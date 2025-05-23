use crate::dataset::{BoardBatch, BoardBatcher, BoardItem, GameResult};
use anyhow::{Result, bail};
use burn::backend::Autodiff;
use burn::backend::ndarray::NdArray;
use burn::data::dataloader::DataLoaderBuilder;
use burn::data::dataset::{Dataset, DatasetIterator};
use burn::nn::Sigmoid;
use burn::nn::loss::MseLoss;
use burn::optim::AdamConfig;
use burn::tensor::Float;
use burn::train::metric::LossMetric;
use burn::train::{RegressionOutput, TrainOutput, TrainStep, ValidStep};
use burn::{
    config::Config,
    module::Module,
    nn::{Linear, LinearConfig},
    tensor::{
        Tensor,
        backend::{AutodiffBackend, Backend},
    },
    train::LearnerBuilder,
};
use burn_ndarray::NdArrayDevice;
use pawnyowl::eval::layers::feature::{PsqFeatureLayer, ScorePair};
use pawnyowl::eval::{model::PsqModel, score::Score};
use pawnyowl_board::{Board, Cell, Color, Sq};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::io::BufReader;
use std::str::FromStr;
use std::{fs::File, io::BufRead};

struct MainDataset {
    items: Vec<BoardItem>,
}

impl MainDataset {
    pub fn new(items: Vec<BoardItem>) -> Self {
        Self { items }
    }
}

impl Dataset<BoardItem> for MainDataset {
    fn get(&self, index: usize) -> Option<BoardItem> {
        self.items.get(index).cloned()
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn iter(&self) -> DatasetIterator<'_, BoardItem>
    where
        Self: Sized,
    {
        DatasetIterator::new(self)
    }
}

#[derive(Config)]
struct TrainingConfig {
    pub model: ModelConfig,
    pub optimizer: AdamConfig,
    #[config(default = 128)]
    pub num_epochs: usize,
    #[config(default = 65536)]
    pub batch_size: usize,
    #[config(default = 0.9)]
    pub train_ratio: f64,
    #[config(default = 4)]
    pub num_workers: usize,
    #[config(default = 42)]
    pub seed: u64,
    #[config(default = 1.0e-2)]
    pub learning_rate: f64,
}

#[derive(Module, Debug)]
struct Model<B: Backend> {
    linear: Linear<B>,
    sigmoid: Sigmoid,
}

#[derive(Config, Debug)]
struct ModelConfig {}

impl ModelConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> Model<B> {
        Model {
            linear: LinearConfig::new(64 * 6, 2).with_bias(false).init(device),
            sigmoid: Sigmoid::new(),
        }
    }
}

impl<B: AutodiffBackend> TrainStep<BoardBatch<B>, RegressionOutput<B>> for Model<B> {
    fn step(&self, batch: BoardBatch<B>) -> TrainOutput<RegressionOutput<B>> {
        let item = self.forward_regression(batch.features, batch.stages, batch.targets);

        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<BoardBatch<B>, RegressionOutput<B>> for Model<B> {
    fn step(&self, batch: BoardBatch<B>) -> RegressionOutput<B> {
        self.forward_regression(batch.features, batch.stages, batch.targets)
    }
}

impl<B: Backend> Model<B> {
    pub fn forward(&self, features: Tensor<B, 2>, stages: Tensor<B, 2>) -> Tensor<B, 2> {
        let res = self.linear.forward(features);
        let n = res.dims()[0];
        let o = res.clone().slice([0..n, 0..1]);
        let e = res.clone().slice([0..n, 1..2]);
        let stage1 = Tensor::<B, 2>::from_floats([[24.0]], &res.device());
        let stage2 = Tensor::<B, 2>::from_floats([[24.0]], &res.device());
        let term1 = o.mul(stages.clone());
        let term2 = e.mul(stage1.sub(stages.clone()));
        let numerator = term1.add(term2);
        self.sigmoid.forward(numerator.div(stage2))
    }
    pub fn forward_regression(
        &self,
        features: Tensor<B, 2>,
        stages: Tensor<B, 2, Float>,
        targets: Tensor<B, 2, Float>,
    ) -> RegressionOutput<B> {
        let output = self.forward(features, stages);
        let loss = MseLoss::new().forward(
            output.clone(),
            targets.clone(),
            burn::nn::loss::Reduction::Auto,
        );

        RegressionOutput::new(loss, output, targets)
    }
}

fn split_last_comma(s: &str) -> (&str, &str) {
    if let Some(last_comma) = s.rfind(',') {
        let (before, after) = s.split_at(last_comma);
        (before, &after[1..])
    } else {
        ("", s)
    }
}

fn parse_result(s: &str) -> Result<GameResult> {
    match s {
        "W" => Ok(GameResult::WhiteWins),
        "D" => Ok(GameResult::Draw),
        "B" => Ok(GameResult::BlackWins),
        _ => bail!("unknown result"),
    }
}

fn read_lines(filename: &str, seed: u64) -> Result<Vec<BoardItem>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let fens: Vec<String> = reader.lines().skip(1).collect::<Result<_, _>>()?;
    let parse_fens = |line: &String| -> Result<_> {
        let (fen, result) = split_last_comma(line);
        let board = Board::from_str(fen)?;

        let mut features = [0_i8; 64 * 6];
        let mut stage = 0;
        for sq in Sq::iter() {
            let cell = board.get(sq);
            if let Some(c) = cell.color() {
                if c == Color::White {
                    features[cell.piece().unwrap().index() * 64 + sq.index()] += 1;
                } else {
                    features[cell.piece().unwrap().index() * 64 + sq.flipped_rank().index()] -= 1;
                }
                stage += PsqFeatureLayer::STAGE_WEIGHTS[cell.index()];
            }
        }
        let target = parse_result(result)?.target();
        Ok(BoardItem {
            features,
            stage,
            target,
        })
    };
    let mut items = fens.iter().map(parse_fens).collect::<Result<Vec<_>>>()?;
    let mut rng = StdRng::seed_from_u64(seed);
    items.shuffle(&mut rng);
    Ok(items)
}

fn split_lines(items: Vec<BoardItem>, ratio: f64) -> (Vec<BoardItem>, Vec<BoardItem>) {
    let mut items = items;

    let split_at = (items.len() as f64 * ratio).round() as usize;
    let second = items.split_off(split_at);

    (items, second)
}

fn train<B: AutodiffBackend>(dataset: &str, artifact: &str, model_path: &str, device: B::Device) {
    let config = TrainingConfig::new(ModelConfig {}, AdamConfig::new());

    let lines = match read_lines(dataset, config.seed) {
        Ok(lines) => {
            println!("Dataset loaded: {} items", lines.len());
            Ok(lines)
        }
        Err(e) => {
            eprintln!("Error loading dataset: {}", e);
            Err(e)
        }
    }
    .unwrap();

    let (items_train, items_valid) = split_lines(lines, config.train_ratio);
    let train_dataset = MainDataset::new(items_train);
    let valid_dataset = MainDataset::new(items_valid);

    let batcher_train = BoardBatcher::<B>::new(device.clone());
    let batcher_valid = BoardBatcher::<B::InnerBackend>::new(device.clone());

    let dataloader_train = DataLoaderBuilder::new(batcher_train)
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(train_dataset);

    let dataloader_valid = DataLoaderBuilder::new(batcher_valid)
        .batch_size(valid_dataset.len())
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(valid_dataset);

    let learner = LearnerBuilder::new(artifact)
        .metric_train_numeric(LossMetric::new())
        .metric_valid_numeric(LossMetric::new())
        .devices(vec![device.clone()])
        .num_epochs(config.num_epochs)
        .summary()
        .build(
            config.model.init::<B>(&device),
            config.optimizer.init(),
            config.learning_rate,
        );

    let model_trained = learner.fit(dataloader_train, dataloader_valid);
    let weights = get_layer_weights(&model_trained.linear);

    let mut o_pawn_weights: Vec<f32> = weights[8..=55].iter().map(|row| row[0]).collect();
    let mut e_pawn_weights: Vec<f32> = weights[8..=55].iter().map(|row| row[1]).collect();
    let o_pawn = median(o_pawn_weights.as_mut_slice());
    let e_pawn = median(e_pawn_weights.as_mut_slice());

    let weights: Vec<Vec<f32>> = weights
        .iter()
        .map(|row| {
            let mut new_row = row.clone();
            new_row[0] /= (o_pawn + e_pawn) / 2.0;
            new_row[0] *= 100.0;
            new_row[1] /= (o_pawn + e_pawn) / 2.0;
            new_row[1] *= 100.0;
            new_row
        })
        .collect();

    let mut feature_layer_weights: [ScorePair; 64 * Cell::COUNT] =
        [ScorePair::new(Score::new(0), Score::new(0)); 64 * Cell::COUNT];
    for cell in Cell::iter() {
        for sq in Sq::iter() {
            if cell == Cell::None {
                continue;
            }
            let weight_pair = match cell.color().unwrap() {
                Color::White => weights[cell.piece().unwrap().index() * 64 + sq.index()].clone(),
                Color::Black => {
                    weights[cell.piece().unwrap().index() * 64 + sq.flipped_rank().index()].clone()
                }
            };
            let score = ScorePair::new(
                Score::new(weight_pair[0].round() as i16),
                Score::new(weight_pair[1].round() as i16),
            );
            feature_layer_weights[PsqFeatureLayer::input_index(cell, sq)] = score;
        }
    }

    let model = PsqModel::from_layers(PsqFeatureLayer::new(feature_layer_weights));
    model.store(model_path).unwrap();
}

pub fn learn_model(dataset: &str, artifact: &str, model_path: &str) {
    type Backend = NdArray<f32>;
    type AutodiffBackend = Autodiff<Backend>;
    let device = NdArrayDevice::Cpu;
    train::<AutodiffBackend>(dataset, artifact, model_path, device);
}

fn median(numbers: &mut [f32]) -> f32 {
    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    numbers[numbers.len() / 2]
}

fn get_layer_weights<B: Backend>(linear_layer: &Linear<B>) -> Vec<Vec<f32>> {
    let weight_data = linear_layer.weight.to_data();

    let shape = weight_data.shape.clone();
    let weights_slice = weight_data.as_slice::<f32>().unwrap();

    let mut weights = Vec::with_capacity(shape[0]);
    for row in 0..shape[0] {
        let mut row_weights = Vec::with_capacity(shape[1]);
        for col in 0..shape[1] {
            row_weights.push(weights_slice[row * shape[1] + col]);
        }
        weights.push(row_weights);
    }

    weights
}
