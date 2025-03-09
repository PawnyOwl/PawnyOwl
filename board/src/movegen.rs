use crate::Bitboard;
use crate::attack;
use crate::board::Board;
use crate::core::{Color, Piece, Sq};

#[inline]
pub fn is_square_attacked(b: &Board, s: Sq, c: Color) -> bool {
    let all = b.all();
    (b.piece(c, Piece::Pawn) & attack::pawn(c.inv(), s)).is_nonempty()
        || (b.piece(c, Piece::King) & attack::king(s)).is_nonempty()
        || (b.piece(c, Piece::Knight) & attack::knight(s)).is_nonempty()
        || (attack::bishop(s, all) & b.piece_diag(c)).is_nonempty()
        || (attack::rook(s, all) & b.piece_line(c)).is_nonempty()
}

#[inline]
pub fn square_attackers(b: &Board, s: Sq, c: Color) -> Bitboard {
    let all = b.all();
    (b.piece(c, Piece::Pawn) & attack::pawn(c.inv(), s))
        | (b.piece(c, Piece::King) & attack::king(s))
        | (b.piece(c, Piece::Knight) & attack::knight(s))
        | (attack::bishop(s, all) & b.piece_diag(c))
        | (attack::rook(s, all) & b.piece_line(c))
}
