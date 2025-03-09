use crate::bitboard::Bitboard;
use crate::core::{Color, File};
use crate::geometry::bitboard;

#[inline]
pub fn advance_forward(c: Color, b: Bitboard) -> Bitboard {
    match c {
        Color::White => b.shr(8),
        Color::Black => b.shl(8),
    }
}

#[inline]
pub fn advance_left(c: Color, b: Bitboard) -> Bitboard {
    let b = b & !bitboard::file(File::A);
    match c {
        Color::White => b.shr(9),
        Color::Black => b.shl(7),
    }
}

#[inline]
pub fn advance_right(c: Color, b: Bitboard) -> Bitboard {
    let b = b & !bitboard::file(File::H);
    match c {
        Color::White => b.shr(7),
        Color::Black => b.shl(9),
    }
}
