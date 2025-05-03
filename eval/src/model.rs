use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{cmp, fs::File, io::Write};

use pawnyowl_board::{
    Board, Cell, Color, Move, Sq,
    diff::{self, DiffListener},
    moves::RawUndo,
};

use crate::{
    layers::feature::{PSQFeatureLayer, PSQFeatureSlice},
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
pub struct PSQModel {
    feature_layer: PSQFeatureLayer,
}

struct PSQListener<'a> {
    model: &'a PSQModel,
    feature_slice: &'a mut PSQFeatureSlice,
}

impl DiffListener for PSQListener<'_> {
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

impl Model for PSQModel {
    type Tag = PSQFeatureSlice;

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
                PSQListener {
                    model: self,
                    feature_slice: tag,
                },
            )
        };
    }

    #[inline]
    fn apply(&self, feature_slice: &PSQFeatureSlice, _move_side: Color) -> Score {
        let clipped_stage =
            cmp::min(feature_slice.stage, PSQFeatureLayer::INIT_STAGE as Stage) as i32;
        Score::from(
            i32::from(feature_slice.score.first()) * clipped_stage
                + i32::from(feature_slice.score.second())
                    * (PSQFeatureLayer::INIT_STAGE as i32 - clipped_stage),
        )
    }
}

impl PSQModel {
    #[inline]
    pub fn from_layers(feature_layer: PSQFeatureLayer) -> Self {
        Self { feature_layer }
    }

    pub fn store(&self, path: &str) -> Result<()> {
        let data = bincode::serialize(&self)?;
        let mut file = File::create(path)?;
        file.write_all(data.as_slice())?;
        Ok(())
    }
}
