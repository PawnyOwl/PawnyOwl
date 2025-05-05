use crate::score::{Score, Stage};
use derive_more::{Add, AddAssign, Sub, SubAssign};
use pawnyowl_board::{Cell, Sq};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::{cmp::Ord, ops::Mul};

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Serialize,
    Deserialize,
)]
pub struct ScorePair(i32);

impl ScorePair {
    #[inline]
    pub fn new(f: Score, s: Score) -> Self {
        ScorePair(f.value() as i32 + (s.value() as i32) * (1 << 16))
    }

    #[inline]
    pub fn first(self) -> Score {
        Score::new(self.0 as i16)
    }

    #[inline]
    pub fn second(self) -> Score {
        let mut res = self.0 >> 16;
        if self.first().value() < 0 {
            res -= 1;
        }
        Score::new(res as i16)
    }
}

impl Mul<i32> for ScorePair {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: i32) -> Self::Output {
        ScorePair(self.0 * scalar)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PSQFeatureSlice {
    pub score: ScorePair,
    pub stage: Stage,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PSQFeatureLayer {
    #[serde(with = "BigArray")]
    weights: [ScorePair; 64 * Cell::COUNT],
}

impl PSQFeatureLayer {
    pub const STAGE_WEIGHTS: [Stage; Cell::COUNT] = [0, 0, 0, 1, 1, 2, 4, 0, 0, 1, 1, 2, 4];
    pub const INIT_STAGE: Stage = 24;

    #[inline]
    pub fn new(weights: [ScorePair; 64 * Cell::COUNT]) -> Self {
        Self { weights }
    }

    #[inline]
    pub fn input_index(cell: Cell, sq: Sq) -> usize {
        cell.index() * 64 + sq.index()
    }

    #[inline]
    pub fn init_feature_slice(&self) -> PSQFeatureSlice {
        PSQFeatureSlice {
            score: ScorePair(0),
            stage: 0,
        }
    }

    #[inline]
    pub fn update_feature_slice(
        &self,
        features: &mut PSQFeatureSlice,
        cell: Cell,
        sq: Sq,
        delta: i32,
    ) {
        features.score += self.weights[Self::input_index(cell, sq)] * delta;
        features.stage += ((Self::STAGE_WEIGHTS[cell.index()] as i32) * delta) as u8;
    }
}
