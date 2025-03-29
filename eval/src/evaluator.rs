use pawnyowl_base::geometry;
use pawnyowl_board::{Board, Move, Piece, File};
use pawnyowl_board::{Cell, MoveKind, Sq};

use crate::{layers::feature::FeatureSlice, model::Model, score::Score};

pub struct EvalBoard<'a> {
    board: Board,
    feature_slice: FeatureSlice,
    model: &'a Model,
}

pub struct RawUndo {
    raw_undo: pawnyowl_board::moves::RawUndo,
    feature_slice: FeatureSlice,
}

impl<'a> EvalBoard<'a> {
    pub fn new(board: Board, model: &'a Model) -> Self {
        let mut res = EvalBoard {
            board,
            feature_slice: FeatureSlice::default(),
            model,
        };
        res.build();
        res
    }
    pub fn score(&self) -> Score {
        self.model.apply(&self.feature_slice, self.board.side())
    }
    pub fn build(&mut self) {
        self.model.init(&mut self.feature_slice);
        for sq in Sq::iter() {
            let cell = self.board.get(sq);
            if cell != Cell::None {
                self.model.update(&mut self.feature_slice, cell, sq, 1);
            }
        }
    }
    pub unsafe fn make_move(&mut self, mv: Move) -> Option<RawUndo> {
        let board_undo = unsafe { self.board.try_make_move_unchecked(mv) }?;
        let raw_undo = RawUndo {
            raw_undo: board_undo,
            feature_slice: self.feature_slice,
        };

        let mut basic_update = |src_cell: Cell, dst_cell: Cell, mv: Move| {
            self.model
                .update(&mut self.feature_slice, src_cell, mv.src(), -1);
            self.model
                .update(&mut self.feature_slice, dst_cell, mv.dst(), -1);
        };

        match mv.kind() {
            MoveKind::Simple => {
                let src_cell = self.board.get(mv.dst());
                basic_update(src_cell, board_undo.dst_cell(), mv);
                self.model.update(
                    &mut self.feature_slice,
                    src_cell,
                    mv.dst(),
                    1,
                );
            }
            MoveKind::PawnSimple => {
                let pawn = Cell::make(self.board.side().inv(), Piece::Pawn);
                basic_update(pawn, board_undo.dst_cell(), mv);
                self.model
                    .update(&mut self.feature_slice, pawn, mv.dst(), 1);
            }
            MoveKind::PawnDouble => {
                let pawn = Cell::make(self.board.side().inv(), Piece::Pawn);
                basic_update(pawn, Cell::None, mv);
                self.model
                    .update(&mut self.feature_slice, pawn, mv.dst(), 1);
            }
            MoveKind::PromoteKnight
            | MoveKind::PromoteBishop
            | MoveKind::PromoteRook
            | MoveKind::PromoteQueen => {
                let pawn = Cell::make(self.board.side().inv(), Piece::Pawn);
                basic_update(pawn, board_undo.dst_cell(), mv);
                self.model.update(
                    &mut self.feature_slice,
                    self.board.get(mv.dst()),
                    mv.dst(),
                    1,
                );
            }
            MoveKind::CastlingKingside => {
                let c = self.board.side().inv();
                let king = Cell::make(c, Piece::King);
                let rook = Cell::make(c, Piece::Rook);
                let rank = geometry::castling_rank(self.board.side().inv());
                self.model.update(
                    &mut self.feature_slice,
                    king,
                    Sq::make(File::E, rank),
                    -1,
                );
                self.model.update(
                    &mut self.feature_slice,
                    king,
                    Sq::make(File::G, rank),
                    1,
                );
                self.model.update(
                    &mut self.feature_slice,
                    rook,
                    Sq::make(File::H, rank),
                    -1,
                );
                self.model.update(
                    &mut self.feature_slice,
                    rook,
                    Sq::make(File::F, rank),
                    1,
                );
            }
            MoveKind::CastlingQueenside => {
                let c = self.board.side().inv();
                let king = Cell::make(c, Piece::King);
                let rook = Cell::make(c, Piece::Rook);
                let rank = geometry::castling_rank(self.board.side().inv());
                self.model.update(
                    &mut self.feature_slice,
                    king,
                    Sq::make(File::E, rank),
                    -1,
                );
                self.model.update(
                    &mut self.feature_slice,
                    king,
                    Sq::make(File::C, rank),
                    1,
                );
                self.model.update(
                    &mut self.feature_slice,
                    rook,
                    Sq::make(File::A, rank),
                    -1,
                );
                self.model.update(
                    &mut self.feature_slice,
                    rook,
                    Sq::make(File::D, rank),
                    1,
                );
            }
            MoveKind::Null => {
                // Do nothing.
            }
            MoveKind::Enpassant => {
                let c = self.board.side().inv();
                let c_inv = c.inv();
                let pawn = Cell::make(c, Piece::Pawn);
                basic_update(pawn, Cell::None, mv);
                self.model
                    .update(&mut self.feature_slice, pawn, mv.dst(), 1);
                let enemy_pawn = Cell::make(c_inv, Piece::Pawn);
                self.model.update(
                    &mut self.feature_slice,
                    enemy_pawn,
                    unsafe {
                        mv.dst()
                            .add_unchecked(geometry::pawn_forward_delta(c_inv))
                    },
                    1,
                );
            }
        }
        Some(raw_undo)
    }
    pub unsafe fn unmake_move(&mut self, mv: Move, raw_undo: RawUndo) {
        unsafe { self.board.unmake_move_unchecked(mv, raw_undo.raw_undo) };
        self.feature_slice = raw_undo.feature_slice;
    }
}
