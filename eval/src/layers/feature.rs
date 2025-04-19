use derive_more::{Add, AddAssign, Sub, SubAssign};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::{cmp::Ord, ops::Mul};

use crate::score::{Score, Stage};
use pawnyowl_board::{Cell, Sq};

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
pub struct FeatureSlice {
    pub score: ScorePair,
    pub stage: Stage,
}

#[derive(Serialize, Deserialize)]
pub struct FeatureLayer {
    #[serde(with = "BigArray")]
    weights: [ScorePair; 64 * Cell::COUNT],
}

impl FeatureLayer {
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
    pub fn init_feature_slice(&self, features: &mut FeatureSlice) {
        features.score = ScorePair(0);
        features.stage = 0;
    }
    #[inline]
    pub fn update_feature_slice(
        &self,
        features: &mut FeatureSlice,
        cell: Cell,
        sq: Sq,
        delta: i32,
    ) {
        features.score += self.weights[Self::input_index(cell, sq)] * delta;
        features.stage += ((Self::STAGE_WEIGHTS[cell.index()] as i32) * delta) as u8;
    }
}
