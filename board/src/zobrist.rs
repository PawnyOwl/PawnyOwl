use crate::core::{CastlingRights, CastlingSide, Cell, Color, Sq};

include!(concat!(env!("OUT_DIR"), "/zobrist.rs"));

#[inline]
pub fn squares(cell: Cell, sq: Sq) -> u64 {
    unsafe {
        *SQUARES
            .get_unchecked(cell.index())
            .get_unchecked(sq.index())
    }
}

#[inline]
pub fn enpassant(sq: Sq) -> u64 {
    unsafe { *ENPASSANT.get_unchecked(sq.index()) }
}

#[inline]
pub fn castling(rights: CastlingRights) -> u64 {
    unsafe { *CASTLING.get_unchecked(rights.index()) }
}

#[inline]
pub fn castling_delta(color: Color, side: CastlingSide) -> u64 {
    match side {
        CastlingSide::Queen => unsafe { *CASTLING_QUEENSIDE.get_unchecked(color as u8 as usize) },
        CastlingSide::King => unsafe { *CASTLING_KINGSIDE.get_unchecked(color as u8 as usize) },
    }
}
