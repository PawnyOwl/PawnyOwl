use crate::Bitboard;
use crate::attack;
use crate::board::Board;
use crate::core::{CastlingSide, Cell, Color, File, Piece, Sq};
use crate::geometry::{self, bitboard};
use crate::moves::{Move, MoveKind};
use crate::{between, castling, generic, pawns};
use arrayvec::ArrayVec;
use std::ops::{Deref, DerefMut};

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

pub trait MovePush {
    fn push(&mut self, m: Move);
}

const GEN_SIMPLE: usize = 1 << 0;
const GEN_CAPTURE: usize = 1 << 1;
const GEN_SIMPLE_PROMOTE: usize = 1 << 2;
const GEN_CASTLING: usize = 1 << 3;
const GEN_MAX: usize = 1 << 4;

#[inline]
fn has_bit(mask: usize, bit: usize) -> bool {
    (mask & bit) != 0
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum CheckKind {
    None,
    Single,
    Double,
}

pub type MoveList = ArrayVec<Move, 256>;

impl<const N: usize> MovePush for ArrayVec<Move, N> {
    #[inline]
    fn push(&mut self, m: Move) {
        self.push(m);
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct UncheckedMoveList<const N: usize>(ArrayVec<Move, N>);

impl<const N: usize> UncheckedMoveList<N> {
    #[inline]
    pub unsafe fn new() -> Self {
        UncheckedMoveList(ArrayVec::new())
    }

    #[inline]
    pub fn get(&self) -> &ArrayVec<Move, N> {
        &self.0
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut ArrayVec<Move, N> {
        &mut self.0
    }
}

impl<const N: usize> Deref for UncheckedMoveList<N> {
    type Target = ArrayVec<Move, N>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for UncheckedMoveList<N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> MovePush for UncheckedMoveList<N> {
    #[inline]
    fn push(&mut self, m: Move) {
        unsafe { self.0.push_unchecked(m) };
    }
}

#[derive(Copy, Clone)]
pub struct MoveGenCtx {
    check_mask: Bitboard,
    check: CheckKind,
    hash: u64,
}

impl From<&Board> for MoveGenCtx {
    fn from(b: &Board) -> Self {
        let king = b.king_pos(b.side());
        let king_attackers = b.checkers();
        let (check, check_mask) = match king_attackers.len() {
            0 => (CheckKind::None, Bitboard::FULL),
            1 => {
                let checker = king_attackers.into_iter().next().unwrap();
                let check_mask = between::between(checker, king) | king_attackers;
                (CheckKind::Single, check_mask)
            }
            _ => (CheckKind::Double, Bitboard::EMPTY),
        };
        Self {
            check_mask,
            check,
            hash: b.zobrist_hash(),
        }
    }
}

pub struct MoveGen<'a> {
    b: &'a Board,
    c: MoveGenCtx,
}

impl<'a> MoveGen<'a> {
    #[inline]
    pub fn new(b: &'a Board) -> Self {
        Self { b, c: b.into() }
    }

    #[inline]
    pub unsafe fn new_unchecked(b: &'a Board, c: &MoveGenCtx) -> Self {
        assert_eq!(b.zobrist_hash(), c.hash);
        Self { b, c: *c }
    }

    #[inline]
    pub fn ctx(&self) -> &MoveGenCtx {
        &self.c
    }

    #[inline(never)]
    fn do_gen2<C: generic::Color, const MASK: usize>(&self, p: &mut impl MovePush) {
        const PROMOTES: [MoveKind; 4] = [
            MoveKind::PromoteKnight,
            MoveKind::PromoteBishop,
            MoveKind::PromoteRook,
            MoveKind::PromoteQueen,
        ];

        let b = self.b;
        let c = C::COLOR;
        let all = b.all();

        if has_bit(MASK, GEN_SIMPLE) || has_bit(MASK, GEN_CAPTURE) {
            let raw_dst_mask = match (has_bit(MASK, GEN_SIMPLE), has_bit(MASK, GEN_CAPTURE)) {
                (true, true) => !b.color(c),
                (true, false) => !all,
                (false, true) => b.color(c.inv()),
                (false, false) => unreachable!(),
            };
            let dst_mask = raw_dst_mask & self.c.check_mask;

            // King
            for s in b.piece(c, Piece::King) {
                for d in attack::king(s) & raw_dst_mask {
                    p.push(unsafe { Move::new_unchecked(MoveKind::Simple, s, d) });
                }
            }

            // Queen
            for s in b.piece(c, Piece::Queen) {
                for d in (attack::rook(s, all) | attack::bishop(s, all)) & dst_mask {
                    p.push(unsafe { Move::new_unchecked(MoveKind::Simple, s, d) });
                }
            }

            // Rook
            for s in b.piece(c, Piece::Rook) {
                for d in attack::rook(s, all) & dst_mask {
                    p.push(unsafe { Move::new_unchecked(MoveKind::Simple, s, d) });
                }
            }

            // Bishop
            for s in b.piece(c, Piece::Bishop) {
                for d in attack::bishop(s, all) & dst_mask {
                    p.push(unsafe { Move::new_unchecked(MoveKind::Simple, s, d) });
                }
            }

            // Knight
            for s in b.piece(c, Piece::Knight) {
                for d in attack::knight(s) & dst_mask {
                    p.push(unsafe { Move::new_unchecked(MoveKind::Simple, s, d) });
                }
            }
        }

        // Pawn
        {
            let pawn = b.piece(c, Piece::Pawn);
            let promote = bitboard::rank(geometry::promote_src_rank(c));

            if has_bit(MASK, GEN_SIMPLE) || has_bit(MASK, GEN_SIMPLE_PROMOTE) {
                let double = bitboard::rank(geometry::double_move_src_rank(c));
                let df = -geometry::pawn_forward_delta(c);
                let dst_mask = self.c.check_mask;

                if has_bit(MASK, GEN_SIMPLE) {
                    // Simple move
                    for d in pawns::advance_forward(c, pawn & !promote) & !all & dst_mask {
                        p.push(unsafe {
                            Move::new_unchecked(MoveKind::PawnSimple, d.add_unchecked(df), d)
                        });
                    }

                    // Double move
                    let pawn_tmp = pawns::advance_forward(c, pawn & double) & !all;
                    for d in pawns::advance_forward(c, pawn_tmp) & !all & dst_mask {
                        p.push(unsafe {
                            Move::new_unchecked(MoveKind::PawnDouble, d.add_unchecked(2 * df), d)
                        });
                    }
                }

                if has_bit(MASK, GEN_SIMPLE_PROMOTE) {
                    // Simple promote
                    for d in pawns::advance_forward(c, pawn & promote) & !all & dst_mask {
                        for pr in PROMOTES {
                            p.push(unsafe { Move::new_unchecked(pr, d.add_unchecked(df), d) });
                        }
                    }
                }
            }

            if has_bit(MASK, GEN_CAPTURE) {
                let dst_mask = b.color(c.inv()) & self.c.check_mask;
                let (dl, dr) = (
                    -geometry::pawn_left_delta(c),
                    -geometry::pawn_right_delta(c),
                );

                // Capture
                {
                    let pawn = pawn & !promote;
                    for d in pawns::advance_left(c, pawn) & dst_mask {
                        p.push(unsafe {
                            Move::new_unchecked(MoveKind::PawnSimple, d.add_unchecked(dl), d)
                        });
                    }
                    for d in pawns::advance_right(c, pawn) & dst_mask {
                        p.push(unsafe {
                            Move::new_unchecked(MoveKind::PawnSimple, d.add_unchecked(dr), d)
                        });
                    }
                }

                // Capture promote
                {
                    let pawn = pawn & promote;
                    for d in pawns::advance_left(c, pawn) & dst_mask {
                        for pr in PROMOTES {
                            p.push(unsafe { Move::new_unchecked(pr, d.add_unchecked(dl), d) });
                        }
                    }
                    for d in pawns::advance_right(c, pawn) & dst_mask {
                        for pr in PROMOTES {
                            p.push(unsafe { Move::new_unchecked(pr, d.add_unchecked(dr), d) });
                        }
                    }
                }

                // En passant
                if let Some(ep) = b.raw().ep_src {
                    let file = ep.file();
                    let dst = unsafe { ep.add_unchecked(geometry::pawn_forward_delta(c)) };
                    let (lp, rp) = unsafe { (ep.add_unchecked(-1), ep.add_unchecked(1)) };
                    let pawn = Cell::make(c, Piece::Pawn);
                    if file != File::A && b.get(lp) == pawn {
                        p.push(unsafe { Move::new_unchecked(MoveKind::Enpassant, lp, dst) });
                    }
                    if file != File::H && b.get(rp) == pawn {
                        p.push(unsafe { Move::new_unchecked(MoveKind::Enpassant, rp, dst) });
                    }
                }
            }
        }

        if has_bit(MASK, GEN_CASTLING)
            && self.c.check == CheckKind::None
            && b.r.castling.has_color(c)
        {
            let rank = geometry::castling_rank(c);
            let inv = c.inv();
            let src = Sq::make(File::E, rank);

            // Queenside castling
            if b.r.castling.has(c, CastlingSide::Queen) {
                let (tmp, dst) = (Sq::make(File::D, rank), Sq::make(File::C, rank));
                if (castling::pass(c, CastlingSide::Queen) & all).is_empty()
                    && !is_square_attacked(b, tmp, inv)
                {
                    p.push(unsafe { Move::new_unchecked(MoveKind::CastlingQueenside, src, dst) });
                }
            }

            // Kingside castling
            if b.r.castling.has(c, CastlingSide::King) {
                let (tmp, dst) = (Sq::make(File::F, rank), Sq::make(File::G, rank));
                if (castling::pass(c, CastlingSide::King) & all).is_empty()
                    && !is_square_attacked(b, tmp, inv)
                {
                    p.push(unsafe { Move::new_unchecked(MoveKind::CastlingKingside, src, dst) });
                }
            }
        }
    }

    #[inline]
    fn do_gen<const MASK: usize>(&self, p: &mut impl MovePush) {
        match self.b.side() {
            Color::White => self.do_gen2::<generic::White, MASK>(p),
            Color::Black => self.do_gen2::<generic::Black, MASK>(p),
        }
    }

    #[inline]
    pub fn gen_all(&self, p: &mut impl MovePush) {
        self.do_gen::<{ GEN_MAX - 1 }>(p)
    }

    #[inline]
    pub fn gen_capture(&self, p: &mut impl MovePush) {
        self.do_gen::<{ GEN_CAPTURE }>(p)
    }

    #[inline]
    pub fn gen_simple(&self, p: &mut impl MovePush) {
        self.do_gen::<{ GEN_SIMPLE | GEN_SIMPLE_PROMOTE | GEN_CASTLING }>(p)
    }

    #[inline]
    pub fn gen_simple_no_promote(&self, p: &mut impl MovePush) {
        self.do_gen::<{ GEN_SIMPLE | GEN_CASTLING }>(p)
    }

    #[inline]
    pub fn gen_simple_promote(&self, p: &mut impl MovePush) {
        self.do_gen::<{ GEN_SIMPLE_PROMOTE }>(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitboard::Bitboard;
    use crate::{Board, Color, File, Rank, Sq};
    use std::str::FromStr;

    #[test]
    fn test_square_attackers() {
        let b = Board::from_str("3R3B/8/3R4/1NP1Q3/3p4/1NP5/5B2/3R1K1k w - - 0 1").unwrap();
        assert!(is_square_attacked(
            &b,
            Sq::make(File::D, Rank::R4),
            Color::White
        ));
        let attackers = Bitboard::EMPTY
            .with2(File::D, Rank::R6)
            .with2(File::B, Rank::R5)
            .with2(File::E, Rank::R5)
            .with2(File::B, Rank::R3)
            .with2(File::C, Rank::R3)
            .with2(File::F, Rank::R2)
            .with2(File::D, Rank::R1);
        assert_eq!(
            square_attackers(&b, Sq::make(File::D, Rank::R4), Color::White),
            attackers
        );
        assert!(!is_square_attacked(
            &b,
            Sq::make(File::D, Rank::R4),
            Color::Black
        ));
        assert_eq!(
            square_attackers(&b, Sq::make(File::D, Rank::R4), Color::Black),
            Bitboard::EMPTY
        );

        let b = Board::from_str("8/8/8/2KPk3/8/8/8/8 w - - 0 1").unwrap();
        assert!(is_square_attacked(
            &b,
            Sq::make(File::D, Rank::R5),
            Color::White
        ));
        assert_eq!(
            square_attackers(&b, Sq::make(File::D, Rank::R5), Color::White),
            Bitboard::EMPTY.with2(File::C, Rank::R5),
        );
        assert!(is_square_attacked(
            &b,
            Sq::make(File::D, Rank::R5),
            Color::Black
        ));
        assert_eq!(
            square_attackers(&b, Sq::make(File::D, Rank::R5), Color::Black),
            Bitboard::EMPTY.with2(File::E, Rank::R5),
        );
    }
}
