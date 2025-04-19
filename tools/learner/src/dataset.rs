use std::str::FromStr;

use itertools::multiunzip;

use burn::{data::dataloader::batcher::Batcher, prelude::*};

use pawnyowl_board::{Cell, Color, Sq};
use pawnyowl_eval::{Board, layers::feature::FeatureLayer};

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

fn split_last_comma(s: &str) -> (String, String) {
    if let Some(last_space) = s.rfind(',') {
        let (before, after) = s.split_at(last_space);
        (before.to_string(), after[1..].to_string())
    } else {
        (String::new(), s.to_string())
    }
}

fn parse_result(s: &str) -> GameResult {
    match s {
        "1" => GameResult::WhiteWins,
        "0.5" => GameResult::Draw,
        "0" => GameResult::BlackWins,
        _ => panic!("Unknown game result"),
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

impl<B: Backend> Batcher<String, BoardBatch<B>> for BoardBatcher<B> {
    fn batch(&self, items: Vec<String>) -> BoardBatch<B> {
        let parse_items = |line: &String| {
            let (fen, result) = split_last_comma(line);
            let board = match Board::from_str(&fen) {
                Ok(board) => board,
                Err(e) => {
                    panic!("{:?}, {}", e, fen);
                }
            };

            let mut values = [0_i8; 64 * 6];
            let mut stage = 0;
            for sq in Sq::iter() {
                let cell = board.get(sq);
                if cell != Cell::None {
                    if cell.color().unwrap() == Color::White {
                        values[cell.piece().unwrap().index() * 64 + sq.index()] += 1;
                    } else {
                        values[cell.piece().unwrap().index() * 64 + sq.flipped_rank().index()] -= 1;
                    }
                    stage += FeatureLayer::STAGE_WEIGHTS[cell.index()];
                }
            }
            let result = parse_result(&result).target();
            (
                Tensor::<B, 2>::from_data(
                    TensorData::from([values; 1]).convert::<B::FloatElem>(),
                    &self.device,
                ),
                Tensor::<B, 2, Float>::from_data(
                    TensorData::from([[stage; 1]; 1]).convert::<B::FloatElem>(),
                    &self.device,
                ),
                Tensor::<B, 2, Float>::from_data(
                    TensorData::from([[result; 1]; 1]).convert::<B::FloatElem>(),
                    &self.device,
                ),
            )
        };

        let (features, stages, targets) =
            multiunzip(items.iter().map(parse_items).collect::<Vec<_>>());

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
