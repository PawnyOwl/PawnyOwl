use crate::bitboard::Bitboard;
use crate::core::{Color, Sq};

#[inline]
const fn bb(val: u64) -> Bitboard {
    Bitboard::from_raw(val)
}

include!(concat!(env!("OUT_DIR"), "/near.rs"));

struct MagicEntry {
    mask: Bitboard,
    post_mask: Bitboard,
    lookup: *const Bitboard,
}

unsafe impl Sync for MagicEntry {}

include!(concat!(env!("OUT_DIR"), "/magic.rs"));

#[inline]
pub fn king(s: Sq) -> Bitboard {
    unsafe { *KING_ATTACKS.get_unchecked(s.index()) }
}

#[inline]
pub fn knight(s: Sq) -> Bitboard {
    unsafe { *KNIGHT_ATTACKS.get_unchecked(s.index()) }
}

#[inline]
pub fn pawn(color: Color, s: Sq) -> Bitboard {
    match color {
        Color::White => unsafe { *WHITE_PAWN_ATTACKS.get_unchecked(s.index()) },
        Color::Black => unsafe { *BLACK_PAWN_ATTACKS.get_unchecked(s.index()) },
    }
}

#[inline]
pub fn rook(s: Sq, occupied: Bitboard) -> Bitboard {
    unsafe {
        let entry = MAGIC_ROOK.get_unchecked(s.index());
        let magic = *MAGIC_CONSTS_ROOK.get_unchecked(s.index());
        let shift = *MAGIC_SHIFTS_ROOK.get_unchecked(s.index());
        let idx = (occupied & entry.mask).as_raw().wrapping_mul(magic) >> shift;
        *entry.lookup.add(idx as usize) & entry.post_mask
    }
}

#[inline]
pub fn bishop(s: Sq, occupied: Bitboard) -> Bitboard {
    unsafe {
        let entry = MAGIC_BISHOP.get_unchecked(s.index());
        let magic = *MAGIC_CONSTS_BISHOP.get_unchecked(s.index());
        let shift = *MAGIC_SHIFTS_BISHOP.get_unchecked(s.index());
        let idx = (occupied & entry.mask).as_raw().wrapping_mul(magic) >> shift;
        *entry.lookup.add(idx as usize) & entry.post_mask
    }
}
