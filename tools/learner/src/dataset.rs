use burn::{data::dataloader::batcher::Batcher, prelude::*};

pub enum GameResult {
    WhiteWins,
    Draw,
    BlackWins,
}

impl GameResult {
    pub fn target(self) -> f64 {
        match self {
            Self::WhiteWins => 1.0,
            Self::Draw => 0.5,
            Self::BlackWins => 0.0,
        }
    }
}

#[derive(Clone)]
pub struct BoardBatcher<B: Backend> {
    device: B::Device,
}

impl<B: Backend> BoardBatcher<B> {
    pub fn new(device: B::Device) -> Self {
        Self { device }
    }
}

#[derive(Clone, Debug)]
pub struct BoardBatch<B: Backend> {
    pub features: Tensor<B, 2>,
    pub stages: Tensor<B, 2, Float>,
    pub targets: Tensor<B, 2, Float>,
}

#[derive(Clone, Debug)]
pub struct BoardItem {
    pub features: [i8; 64 * 6],
    pub stage: u8,
    pub target: f64,
}

impl<B: Backend> Batcher<BoardItem, BoardBatch<B>> for BoardBatcher<B> {
    fn batch(&self, items: Vec<BoardItem>) -> BoardBatch<B> {
        let parse_items = |item: &BoardItem| {
            (
                Tensor::<B, 2>::from_data(
                    TensorData::from([item.features; 1]).convert::<B::FloatElem>(),
                    &self.device,
                ),
                Tensor::<B, 2, Float>::from_data(
                    TensorData::from([[item.stage; 1]; 1]).convert::<B::FloatElem>(),
                    &self.device,
                ),
                Tensor::<B, 2, Float>::from_data(
                    TensorData::from([[item.target; 1]; 1]).convert::<B::FloatElem>(),
                    &self.device,
                ),
            )
        };

        let (features, stages, targets) =
            itertools::multiunzip(items.iter().map(parse_items).collect::<Vec<_>>());

        let features = Tensor::cat(features, 0).to_device(&self.device);
        let stages = Tensor::cat(stages, 0).to_device(&self.device);
        let targets = Tensor::cat(targets, 0).to_device(&self.device);

        BoardBatch {
            features,
            stages,
            targets,
        }
    }
}
