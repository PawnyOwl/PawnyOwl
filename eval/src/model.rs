use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{cmp, fs::File, io::Write};
use pawnyowl_board::{
    Board, Cell, Color, Move, Sq,
    diff::{self, DiffListener},
    moves::RawUndo,
};
use crate::{
    layers::feature::{PsqFeatureLayer, PsqFeatureSlice},
    score::{Score, Stage},
};

pub trait Model: Sized {
    type Tag;

    fn new() -> Self;
    fn build_tag(&self, board: &Board) -> Self::Tag;
    unsafe fn after_move(&self, tag: &mut Self::Tag, board: &Board, mv: Move, u: &RawUndo);
    fn apply(&self, tag: &Self::Tag, move_side: Color) -> Score;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PsqModel {
    feature_layer: PsqFeatureLayer,
}

struct PsqListener<'a> {
    model: &'a PsqModel,
    feature_slice: &'a mut PsqFeatureSlice,
}

impl DiffListener for PsqListener<'_> {
    #[inline]
    fn upd(&mut self, sq: Sq, old: Cell, new: Cell) {
        self.model
            .feature_layer
            .update_feature_slice(self.feature_slice, old, sq, -1);
        self.model
            .feature_layer
            .update_feature_slice(self.feature_slice, new, sq, 1);
    }

    #[inline]
    fn add(&mut self, sq: Sq, new: Cell) {
        self.model
            .feature_layer
            .update_feature_slice(self.feature_slice, new, sq, 1);
    }

    #[inline]
    fn del(&mut self, sq: Sq, old: Cell) {
        self.model
            .feature_layer
            .update_feature_slice(self.feature_slice, old, sq, -1);
    }
}

impl Model for PsqModel {
    type Tag = PsqFeatureSlice;

    #[inline]
    fn new() -> Self {
        let bytes = include_bytes!("../../incbin/model.paw");
        bincode::deserialize(bytes).unwrap()
    }

    #[inline]
    fn build_tag(&self, board: &Board) -> Self::Tag {
        let mut feature_slice = self.feature_layer.init_feature_slice();
        for sq in Sq::iter() {
            let cell = board.get(sq);
            if cell != Cell::None {
                self.feature_layer
                    .update_feature_slice(&mut feature_slice, cell, sq, 1);
            }
        }
        feature_slice
    }

    #[inline]
    unsafe fn after_move(&self, tag: &mut Self::Tag, board: &Board, mv: Move, u: &RawUndo) {
        unsafe {
            diff::after_move(
                board,
                mv,
                u,
                PsqListener {
                    model: self,
                    feature_slice: tag,
                },
            )
        };
    }

    #[inline]
    fn apply(&self, feature_slice: &PsqFeatureSlice, _move_side: Color) -> Score {
        let clipped_stage =
            cmp::min(feature_slice.stage, PsqFeatureLayer::INIT_STAGE as Stage) as i32;
        Score::from(
            i32::from(feature_slice.score.first()) * clipped_stage
                + i32::from(feature_slice.score.second())
                    * (PsqFeatureLayer::INIT_STAGE as i32 - clipped_stage),
        )
    }
}

impl PsqModel {
    #[inline]
    pub fn from_layers(feature_layer: PsqFeatureLayer) -> Self {
        Self { feature_layer }
    }

    pub fn store(&self, path: &str) -> Result<()> {
        let data = bincode::serialize(&self)?;
        let mut file = File::create(path)?;
        file.write_all(data.as_slice())?;
        Ok(())
    }
}
