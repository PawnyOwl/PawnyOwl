use std::io::BufReader;
use std::{fs::File, io::BufRead};

use burn::backend::Autodiff;
use burn::backend::ndarray::NdArray;
use burn::data::dataloader::DataLoaderBuilder;
use burn::data::dataset::Dataset;
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

use crate::dataset::{BoardBatch, BoardBatcher};

struct MainDataset {
    items: Vec<String>,
}

impl MainDataset {
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }
}

impl Dataset<String> for MainDataset {
    fn get(&self, index: usize) -> Option<String> {
        self.items.get(index).cloned()
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn iter(&self) -> burn::data::dataset::DatasetIterator<'_, String>
    where
        Self: Sized,
    {
        burn::data::dataset::DatasetIterator::new(self)
    }
}

fn read_lines(filename: &str) -> Result<Vec<String>, std::io::Error> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    reader.lines().collect()
}

fn split_lines(items: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut items = items;

    let split_at = items.len() * 99 / 100;
    let second = items.split_off(split_at);

    (items, second)
}

#[derive(Config)]
struct TrainingConfig {
    pub model: ModelConfig,
    pub optimizer: AdamConfig,
    #[config(default = 128)]
    pub num_epochs: usize,
    #[config(default = 32768)]
    pub batch_size: usize,
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
struct ModelConfig {
    _unused: bool,
}

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

fn train<B: AutodiffBackend>(dataset: &str, artifact: &str, device: B::Device) {
    let lines = match read_lines(dataset) {
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

    let (lines_train, lines_valid) = split_lines(lines);
    let train_dataset = MainDataset::new(lines_train);
    let valid_dataset = MainDataset::new(lines_valid);

    let batcher_train = BoardBatcher::<B>::new(device.clone());
    let batcher_valid = BoardBatcher::<B::InnerBackend>::new(device.clone());

    let config = TrainingConfig::new(ModelConfig { _unused: false }, AdamConfig::new());

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

    let _model_trained = learner.fit(dataloader_train, dataloader_valid);
    let weights = get_layer_weights(&_model_trained.linear);

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

    println!("{:?}", weights.to_vec());
    println!("{:?} {:?}", o_pawn, e_pawn);
}

pub fn learn_model(dataset: &str, artifact: &str) {
    type Backend = NdArray<f32>;
    type AutodiffBackend = Autodiff<Backend>;
    let device = NdArrayDevice::Cpu;
    train::<AutodiffBackend>(dataset, artifact, device);
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
