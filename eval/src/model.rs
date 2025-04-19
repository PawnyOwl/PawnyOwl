use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{cmp::min, fs::File, io::Write};

use pawnyowl_board::{Cell, Color, Sq};

use crate::{
    layers::feature::{FeatureLayer, FeatureSlice},
    score::{Score, Stage},
};

#[derive(Serialize, Deserialize)]
pub struct Model {
    feature_layer: FeatureLayer,
}

impl Model {
    #[inline]
    pub fn new() -> Result<Self> {
        let bytes = include_bytes!("../../incbin/model.paw");
        Ok(bincode::deserialize(bytes)?)
    }
    #[inline]
    pub fn from_layers(feature_layer: FeatureLayer) -> Self {
        Self { feature_layer }
    }
    #[inline]
    pub fn init(&self, feature_slice: &mut FeatureSlice) {
        self.feature_layer.init_feature_slice(feature_slice);
    }
    #[inline]
    pub fn update(&self, feature_slice: &mut FeatureSlice, cell: Cell, sq: Sq, delta: i32) {
        self.feature_layer
            .update_feature_slice(feature_slice, cell, sq, delta);
    }
    #[inline]
    pub fn apply(&self, feature_slice: &FeatureSlice, _move_side: Color) -> Score {
        let clipped_stage = min(feature_slice.stage, FeatureLayer::INIT_STAGE as Stage) as i32;
        Score::from(
            i32::from(feature_slice.score.first()) * clipped_stage
                + i32::from(feature_slice.score.second())
                    * (FeatureLayer::INIT_STAGE as i32 - clipped_stage),
        )
    }

    pub fn store(&self, path: &str) -> Result<()> {
        let data = bincode::serialize(&self)?;
        let mut file = File::create(path)?;
        file.write_all(data.as_slice())?;
        Ok(())
    }
}
