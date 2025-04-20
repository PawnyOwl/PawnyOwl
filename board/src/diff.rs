use crate::{
    board::Board,
    core::{Cell, Sq},
    moves::{self, Move, RawUndo},
};

pub trait DiffListener {
    fn upd(&mut self, sq: Sq, old: Cell, new: Cell);

    #[inline]
    fn add(&mut self, sq: Sq, new: Cell) {
        self.upd(sq, Cell::None, new)
    }

    #[inline]
    fn del(&mut self, sq: Sq, old: Cell) {
        self.upd(sq, old, Cell::None)
    }
}

#[inline]
pub unsafe fn after_move(b: &Board, mv: Move, u: &RawUndo, l: &mut (impl DiffListener + ?Sized)) {
    unsafe { moves::diff_after_move(b, mv, u, l) }
}
