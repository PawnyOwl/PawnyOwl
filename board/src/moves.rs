use crate::bitboard::Bitboard;
use crate::board::Board;
use crate::core::{CastlingRights, CastlingSide, Cell, Color, File, Piece, Rank, Sq, SqParseError};
use crate::diff::DiffListener;
use crate::{attack, between, castling, generic, geometry, movegen, pawns, zobrist};
use std::str::FromStr;
use std::{fmt, hint};
use thiserror::Error;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MoveKind {
    #[default]
    Null = 0,
    Simple = 1,
    CastlingKingside = 2,
    CastlingQueenside = 3,
    PawnSimple = 4,
    PawnDouble = 5,
    Enpassant = 6,
    PromoteKnight = 7,
    PromoteBishop = 8,
    PromoteRook = 9,
    PromoteQueen = 10,
}

impl MoveKind {
    pub const COUNT: usize = 11;

    #[inline]
    pub const fn index(self) -> usize {
        self as u8 as usize
    }

    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..Self::COUNT).map(|x| unsafe { Self::from_index_unchecked(x) })
    }

    #[inline]
    pub const unsafe fn from_index_unchecked(val: usize) -> Self {
        match val {
            0 => Self::Null,
            1 => Self::Simple,
            2 => Self::CastlingKingside,
            3 => Self::CastlingQueenside,
            4 => Self::PawnSimple,
            5 => Self::PawnDouble,
            6 => Self::Enpassant,
            7 => Self::PromoteKnight,
            8 => Self::PromoteBishop,
            9 => Self::PromoteRook,
            10 => Self::PromoteQueen,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    #[inline]
    pub const fn from_index(val: usize) -> Self {
        assert!(val < Self::COUNT, "index too large");
        unsafe { Self::from_index_unchecked(val) }
    }
}

impl From<CastlingSide> for MoveKind {
    #[inline]
    fn from(side: CastlingSide) -> Self {
        match side {
            CastlingSide::King => Self::CastlingKingside,
            CastlingSide::Queen => Self::CastlingQueenside,
        }
    }
}

impl TryFrom<MoveKind> for CastlingSide {
    type Error = ();

    #[inline]
    fn try_from(kind: MoveKind) -> Result<Self, Self::Error> {
        match kind {
            MoveKind::CastlingKingside => Ok(Self::King),
            MoveKind::CastlingQueenside => Ok(Self::Queen),
            _ => Err(()),
        }
    }
}

impl MoveKind {
    #[inline]
    pub fn promote_with(p: Piece) -> Option<Self> {
        match p {
            Piece::Knight => Some(Self::PromoteKnight),
            Piece::Bishop => Some(Self::PromoteBishop),
            Piece::Rook => Some(Self::PromoteRook),
            Piece::Queen => Some(Self::PromoteQueen),
            _ => None,
        }
    }

    #[inline]
    pub fn promote(self) -> Option<Piece> {
        match self {
            Self::PromoteKnight => Some(Piece::Knight),
            Self::PromoteBishop => Some(Piece::Bishop),
            Self::PromoteRook => Some(Piece::Rook),
            Self::PromoteQueen => Some(Piece::Queen),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Move {
    kind: MoveKind,
    src: Sq,
    dst: Sq,
    unused: u8,
}

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum ValidateError {
    #[error("move is not well-formed")]
    NotWellFormed,
    #[error("move is not semi-legal")]
    NotSemiLegal,
    #[error("move is not legal")]
    NotLegal,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PackedMove(u16);

impl PackedMove {
    pub fn value(self) -> u16 {
        self.0
    }
}

impl From<Move> for PackedMove {
    fn from(m: Move) -> Self {
        let val = (m.kind.index() << 12) | (m.src.index() << 6) | m.dst.index();
        PackedMove(val as u16)
    }
}

impl From<PackedMove> for Move {
    fn from(m: PackedMove) -> Self {
        let val = m.0 as usize;
        let dst = val & 63;
        let src = (val >> 6) & 63;
        let kind = val >> 12;
        unsafe {
            Move {
                kind: MoveKind::from_index_unchecked(kind),
                src: Sq::from_index_unchecked(src),
                dst: Sq::from_index_unchecked(dst),
                unused: 0,
            }
        }
    }
}

impl Move {
    pub const NULL: Move = Move {
        kind: MoveKind::Null,
        src: Sq::from_index(0),
        dst: Sq::from_index(0),
        unused: 0,
    };

    #[inline]
    pub fn from_castling(color: Color, side: CastlingSide) -> Move {
        let rank = geometry::castling_rank(color);
        let src = Sq::make(File::E, rank);
        let dst = match side {
            CastlingSide::King => Sq::make(File::G, rank),
            CastlingSide::Queen => Sq::make(File::C, rank),
        };
        Move {
            kind: MoveKind::from(side),
            src,
            dst,
            unused: 0,
        }
    }

    #[inline]
    pub const unsafe fn new_unchecked(kind: MoveKind, src: Sq, dst: Sq) -> Move {
        Move {
            kind,
            src,
            dst,
            unused: 0,
        }
    }

    #[inline]
    pub fn from_uci(s: &str, b: &Board) -> Result<Self, UciParseError> {
        let u = UciMove::from_str(s)?;
        Ok(u.into_move(b)?)
    }

    #[inline]
    pub fn from_uci_legal(s: &str, b: &Board) -> Result<Self, UciParseError> {
        let m = Self::from_uci(s, b)?;
        m.validate(b)?;
        Ok(m)
    }

    #[inline]
    pub fn is_semilegal(self, b: &Board) -> bool {
        match b.r.side {
            Color::White => do_is_move_semilegal::<generic::White>(b, self),
            Color::Black => do_is_move_semilegal::<generic::Black>(b, self),
        }
    }

    #[inline]
    pub unsafe fn is_legal_unchecked(self, b: &Board) -> bool {
        match b.r.side {
            Color::White => do_is_move_legal::<generic::White>(b, self),
            Color::Black => do_is_move_legal::<generic::Black>(b, self),
        }
    }

    #[inline]
    pub fn semi_validate(self, b: &Board) -> Result<(), ValidateError> {
        if !self.is_semilegal(b) {
            return Err(ValidateError::NotSemiLegal);
        }
        Ok(())
    }

    #[inline]
    pub fn validate(self, b: &Board) -> Result<(), ValidateError> {
        self.semi_validate(b)?;
        match unsafe { self.is_legal_unchecked(b) } {
            true => Ok(()),
            false => Err(ValidateError::NotLegal),
        }
    }

    pub fn new(kind: MoveKind, src: Sq, dst: Sq) -> Result<Move, ValidateError> {
        let mv = Move {
            kind,
            src,
            dst,
            unused: 0,
        };
        if !mv.is_well_formed() {
            return Err(ValidateError::NotWellFormed);
        }
        Ok(mv)
    }

    pub fn is_well_formed(self) -> bool {
        if self.kind == MoveKind::Null {
            return self == Move::NULL;
        }

        if self.src == self.dst {
            return false;
        }

        match self.kind {
            MoveKind::Simple => true,
            MoveKind::CastlingKingside => [Color::White, Color::Black].into_iter().any(|c| {
                let rank = geometry::castling_rank(c);
                self.src == Sq::make(File::E, rank) && self.dst == Sq::make(File::G, rank)
            }),
            MoveKind::CastlingQueenside => [Color::White, Color::Black].into_iter().any(|c| {
                let rank = geometry::castling_rank(c);
                self.src == Sq::make(File::E, rank) && self.dst == Sq::make(File::C, rank)
            }),
            MoveKind::PawnSimple => {
                self.src.file().index().abs_diff(self.dst.file().index()) <= 1
                    && !matches!(self.src.rank(), Rank::R1 | Rank::R8)
                    && !matches!(self.dst.rank(), Rank::R1 | Rank::R8)
                    && self.src.rank().index().abs_diff(self.dst.rank().index()) == 1
            }
            MoveKind::PawnDouble => {
                self.src.file() == self.dst.file()
                    && [Color::White, Color::Black].into_iter().any(|c| {
                        self.src.rank() == geometry::double_move_src_rank(c)
                            && self.dst.rank() == geometry::double_move_dst_rank(c)
                    })
            }
            MoveKind::Enpassant => {
                self.src.file().index().abs_diff(self.dst.file().index()) == 1
                    && [Color::White, Color::Black].into_iter().any(|c| {
                        self.src.rank() == geometry::ep_src_rank(c)
                            && self.dst.rank() == geometry::ep_dst_rank(c)
                    })
            }
            MoveKind::PromoteKnight
            | MoveKind::PromoteBishop
            | MoveKind::PromoteRook
            | MoveKind::PromoteQueen => {
                self.src.file().index().abs_diff(self.dst.file().index()) <= 1
                    && [Color::White, Color::Black].into_iter().any(|c| {
                        self.src.rank() == geometry::promote_src_rank(c)
                            && self.dst.rank() == geometry::promote_dst_rank(c)
                    })
            }
            MoveKind::Null => unreachable!(),
        }
    }

    #[inline]
    pub const fn kind(self) -> MoveKind {
        self.kind
    }

    #[inline]
    pub const fn src(self) -> Sq {
        self.src
    }

    #[inline]
    pub const fn dst(self) -> Sq {
        self.dst
    }
}

impl Default for Move {
    #[inline]
    fn default() -> Self {
        Move::NULL
    }
}

impl fmt::Display for Move {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self.kind {
            MoveKind::Null => write!(f, "0000"),
            _ => {
                write!(f, "{}{}", self.src, self.dst)?;
                match self.kind.promote() {
                    Some(Piece::Knight) => write!(f, "n")?,
                    Some(Piece::Bishop) => write!(f, "b")?,
                    Some(Piece::Rook) => write!(f, "r")?,
                    Some(Piece::Queen) => write!(f, "q")?,
                    Some(Piece::Pawn) | Some(Piece::King) => unreachable!(),
                    None => {}
                };
                Ok(())
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RawUndo {
    hash: u64,
    dst_cell: Cell,
    castling: CastlingRights,
    ep_src: Option<Sq>,
    move_counter: u16,
}

impl RawUndo {
    #[inline]
    pub fn dst_cell(&self) -> Cell {
        self.dst_cell
    }
}

fn update_castling(b: &mut Board, change: Bitboard) {
    if (change & castling::ALL_SRCS).is_empty() {
        return;
    }

    let mut castling = b.r.castling;
    for (c, s) in [
        (Color::White, CastlingSide::Queen),
        (Color::White, CastlingSide::King),
        (Color::Black, CastlingSide::Queen),
        (Color::Black, CastlingSide::King),
    ] {
        if (change & castling::srcs(c, s)).is_nonempty() {
            castling.unset(c, s);
        }
    }

    if castling != b.r.castling {
        b.hash ^= zobrist::castling(b.r.castling);
        b.r.castling = castling;
        b.hash ^= zobrist::castling(b.r.castling);
    }
}

#[inline(always)]
fn do_make_pawn_double(b: &mut Board, mv: Move, change: Bitboard, c: Color, inv: bool) {
    let pawn = Cell::make(c, Piece::Pawn);
    if inv {
        b.r.put(mv.src, pawn);
        b.r.put(mv.dst, Cell::None);
    } else {
        b.r.put(mv.src, Cell::None);
        b.r.put(mv.dst, pawn);
        b.hash ^= zobrist::squares(pawn, mv.src) ^ zobrist::squares(pawn, mv.dst);
    }
    *b.color_mut(c) ^= change;
    *b.cell_mut(pawn) ^= change;
    if !inv {
        b.r.ep_src = Some(mv.dst);
        b.hash ^= zobrist::enpassant(mv.dst);
    }
}

#[inline(always)]
fn do_make_enpassant(b: &mut Board, mv: Move, change: Bitboard, c: Color, inv: bool) {
    let taken_pos = unsafe { mv.dst.add_unchecked(-geometry::pawn_forward_delta(c)) };
    let taken = Bitboard::one(taken_pos);
    let our_pawn = Cell::make(c, Piece::Pawn);
    let their_pawn = Cell::make(c.inv(), Piece::Pawn);
    if inv {
        b.r.put(mv.src, our_pawn);
        b.r.put(mv.dst, Cell::None);
        b.r.put(taken_pos, their_pawn);
    } else {
        b.r.put(mv.src, Cell::None);
        b.r.put(mv.dst, our_pawn);
        b.r.put(taken_pos, Cell::None);
        b.hash ^= zobrist::squares(our_pawn, mv.src)
            ^ zobrist::squares(our_pawn, mv.dst)
            ^ zobrist::squares(their_pawn, taken_pos);
    }
    *b.color_mut(c) ^= change;
    *b.cell_mut(our_pawn) ^= change;
    *b.color_mut(c.inv()) ^= taken;
    *b.cell_mut(their_pawn) ^= taken;
}

#[inline(always)]
fn do_make_castling_kingside(b: &mut Board, c: Color, inv: bool) {
    let king = Cell::make(c, Piece::King);
    let rook = Cell::make(c, Piece::Rook);
    let rank = geometry::castling_rank(c);
    if inv {
        b.r.put2(File::E, rank, king);
        b.r.put2(File::F, rank, Cell::None);
        b.r.put2(File::G, rank, Cell::None);
        b.r.put2(File::H, rank, rook);
    } else {
        b.r.put2(File::E, rank, Cell::None);
        b.r.put2(File::F, rank, rook);
        b.r.put2(File::G, rank, king);
        b.r.put2(File::H, rank, Cell::None);
        b.hash ^= zobrist::castling_delta(c, CastlingSide::King);
    }
    let off = castling::offset(c);
    *b.color_mut(c) ^= Bitboard::from(0xf0 << off);
    *b.cell_mut(rook) ^= Bitboard::from(0xa0 << off);
    *b.cell_mut(king) ^= Bitboard::from(0x50 << off);
    if !inv {
        b.hash ^= zobrist::castling(b.r.castling);
        b.r.castling.unset_color(c);
        b.hash ^= zobrist::castling(b.r.castling);
    }
}

#[inline(always)]
fn do_make_castling_queenside(b: &mut Board, c: Color, inv: bool) {
    let king = Cell::make(c, Piece::King);
    let rook = Cell::make(c, Piece::Rook);
    let rank = geometry::castling_rank(c);
    if inv {
        b.r.put2(File::A, rank, rook);
        b.r.put2(File::C, rank, Cell::None);
        b.r.put2(File::D, rank, Cell::None);
        b.r.put2(File::E, rank, king);
    } else {
        b.r.put2(File::A, rank, Cell::None);
        b.r.put2(File::C, rank, king);
        b.r.put2(File::D, rank, rook);
        b.r.put2(File::E, rank, Cell::None);
        b.hash ^= zobrist::castling_delta(c, CastlingSide::Queen);
    }
    let off = castling::offset(c);
    *b.color_mut(c) ^= Bitboard::from_raw(0x1d << off);
    *b.cell_mut(rook) ^= Bitboard::from_raw(0x09 << off);
    *b.cell_mut(king) ^= Bitboard::from_raw(0x14 << off);
    if !inv {
        b.hash ^= zobrist::castling(b.r.castling);
        b.r.castling.unset_color(c);
        b.hash ^= zobrist::castling(b.r.castling);
    }
}

#[inline(never)]
fn do_make_move<C: generic::Color>(b: &mut Board, mv: Move) -> RawUndo {
    let c = C::COLOR;
    let src_cell = b.get(mv.src);
    let dst_cell = b.get(mv.dst);
    let undo = RawUndo {
        hash: b.hash,
        dst_cell,
        castling: b.r.castling,
        ep_src: b.r.ep_src,
        move_counter: b.r.move_counter,
    };
    let src = Bitboard::one(mv.src);
    let dst = Bitboard::one(mv.dst);
    let change = src | dst;
    let pawn = Cell::make(c, Piece::Pawn);
    if let Some(p) = b.r.ep_src {
        b.hash ^= zobrist::enpassant(p);
        b.r.ep_src = None;
    }
    match mv.kind {
        MoveKind::Simple | MoveKind::PawnSimple => {
            b.r.put(mv.src, Cell::None);
            b.r.put(mv.dst, src_cell);
            b.hash ^= zobrist::squares(src_cell, mv.src)
                ^ zobrist::squares(src_cell, mv.dst)
                ^ zobrist::squares(dst_cell, mv.dst);
            *b.color_mut(c) ^= change;
            *b.cell_mut(src_cell) ^= change;
            *b.color_mut(c.inv()) &= !dst;
            *b.cell_mut(dst_cell) &= !dst;
            if src_cell != pawn {
                update_castling(b, change);
            }
        }
        MoveKind::PawnDouble => {
            do_make_pawn_double(b, mv, change, c, false);
        }
        MoveKind::PromoteKnight
        | MoveKind::PromoteBishop
        | MoveKind::PromoteRook
        | MoveKind::PromoteQueen => {
            let promote = Cell::make(c, mv.kind.promote().unwrap());
            b.r.put(mv.src, Cell::None);
            b.r.put(mv.dst, promote);
            b.hash ^= zobrist::squares(src_cell, mv.src)
                ^ zobrist::squares(promote, mv.dst)
                ^ zobrist::squares(dst_cell, mv.dst);
            *b.color_mut(c) ^= change;
            *b.cell_mut(pawn) ^= src;
            *b.cell_mut(promote) ^= dst;
            *b.color_mut(c.inv()) &= !dst;
            *b.cell_mut(dst_cell) &= !dst;
            update_castling(b, change);
        }
        MoveKind::CastlingKingside => {
            do_make_castling_kingside(b, c, false);
        }
        MoveKind::CastlingQueenside => {
            do_make_castling_queenside(b, c, false);
        }
        MoveKind::Null => {
            // Do nothing.
        }
        MoveKind::Enpassant => {
            do_make_enpassant(b, mv, change, c, false);
        }
    }

    if dst_cell != Cell::None || src_cell == pawn {
        b.r.move_counter = 0;
    } else {
        b.r.move_counter += 1;
    }

    b.r.side = c.inv();
    b.hash ^= zobrist::MOVE_SIDE;
    if c == Color::Black {
        b.r.move_number += 1;
    }
    b.all_v = b.white | b.black;

    undo
}

#[inline]
pub(crate) unsafe fn make_move_unchecked(b: &mut Board, mv: Move) -> RawUndo {
    match b.r.side {
        Color::White => do_make_move::<generic::White>(b, mv),
        Color::Black => do_make_move::<generic::Black>(b, mv),
    }
}

#[inline(never)]
fn do_unmake_move<C: generic::Color>(b: &mut Board, mv: Move, u: RawUndo) {
    let c = C::COLOR;
    let src = Bitboard::one(mv.src);
    let dst = Bitboard::one(mv.dst);
    let change = src | dst;
    let src_cell = b.get(mv.dst);
    let dst_cell = u.dst_cell;

    match mv.kind {
        MoveKind::Simple | MoveKind::PawnSimple => {
            b.r.put(mv.src, src_cell);
            b.r.put(mv.dst, dst_cell);
            *b.color_mut(c) ^= change;
            *b.cell_mut(src_cell) ^= change;
            if dst_cell != Cell::None {
                *b.color_mut(c.inv()) |= dst;
                *b.cell_mut(dst_cell) |= dst;
            }
        }
        MoveKind::PawnDouble => {
            do_make_pawn_double(b, mv, change, c, true);
        }
        MoveKind::PromoteKnight
        | MoveKind::PromoteBishop
        | MoveKind::PromoteRook
        | MoveKind::PromoteQueen => {
            let pawn = Cell::make(c, Piece::Pawn);
            b.r.put(mv.src, pawn);
            b.r.put(mv.dst, dst_cell);
            *b.color_mut(c) ^= change;
            *b.cell_mut(pawn) ^= src;
            *b.cell_mut(src_cell) ^= dst;
            if dst_cell != Cell::None {
                *b.color_mut(c.inv()) |= dst;
                *b.cell_mut(dst_cell) |= dst;
            }
        }
        MoveKind::CastlingKingside => {
            do_make_castling_kingside(b, c, true);
        }
        MoveKind::CastlingQueenside => {
            do_make_castling_queenside(b, c, true);
        }
        MoveKind::Null => {
            // Do nothing.
        }
        MoveKind::Enpassant => {
            do_make_enpassant(b, mv, change, c, true);
        }
    }

    b.hash = u.hash;
    b.r.castling = u.castling;
    b.r.ep_src = u.ep_src;
    b.r.move_counter = u.move_counter;
    b.r.side = c;
    if c == Color::Black {
        b.r.move_number -= 1;
    }
    b.all_v = b.white | b.black;
}

#[inline]
pub(crate) unsafe fn unmake_move_unchecked(b: &mut Board, mv: Move, u: RawUndo) {
    match b.r.side {
        Color::White => do_unmake_move::<generic::Black>(b, mv, u),
        Color::Black => do_unmake_move::<generic::White>(b, mv, u),
    }
}

#[inline(always)]
fn is_bishop_semilegal(src: Sq, dst: Sq, all: Bitboard) -> bool {
    between::is_bishop_valid(src, dst) && (between::bishop_strict(src, dst) & all).is_empty()
}

#[inline(always)]
fn is_rook_semilegal(src: Sq, dst: Sq, all: Bitboard) -> bool {
    between::is_rook_valid(src, dst) && (between::rook_strict(src, dst) & all).is_empty()
}

#[inline(always)]
fn is_queen_semilegal(src: Sq, dst: Sq, all: Bitboard) -> bool {
    is_bishop_semilegal(src, dst, all) || is_rook_semilegal(src, dst, all)
}

#[inline(never)]
fn do_is_move_semilegal<C: generic::Color>(b: &Board, mv: Move) -> bool {
    if mv.kind == MoveKind::Null {
        return false;
    }

    let c = C::COLOR;
    let src_cell = b.get(mv.src);

    if src_cell == Cell::make(c, Piece::Pawn) {
        if match c {
            Color::White => mv.src.index() <= mv.dst.index(),
            Color::Black => mv.src.index() >= mv.dst.index(),
        } {
            return false;
        }
        return match mv.kind {
            MoveKind::PawnDouble => {
                let must_empty = match c {
                    Color::White => Bitboard::from(0x0101_u64 << (mv.src.index() - 16)),
                    Color::Black => Bitboard::from(0x010100_u64 << mv.src.index()),
                };
                (b.all() & must_empty).is_empty()
            }
            MoveKind::Enpassant => match b.r.ep_src {
                Some(p) => unsafe {
                    (p == mv.src.add_unchecked(1) || p == mv.src.add_unchecked(-1))
                        && mv.dst == p.add_unchecked(geometry::pawn_forward_delta(c))
                },
                None => false,
            },
            MoveKind::PawnSimple
            | MoveKind::PromoteKnight
            | MoveKind::PromoteBishop
            | MoveKind::PromoteRook
            | MoveKind::PromoteQueen => {
                let dst_cell = b.get(mv.dst);
                if let Some(cc) = dst_cell.color() {
                    c != cc && mv.dst.file() != mv.src.file()
                } else {
                    mv.dst.file() == mv.src.file()
                }
            }
            _ => false,
        };
    }

    match mv.kind {
        MoveKind::Simple => {
            let dst_cell = b.get(mv.dst);
            if src_cell.color() != Some(c) || dst_cell.color() == Some(c) {
                return false;
            }
            let dst = Bitboard::one(mv.dst);
            match src_cell.piece() {
                Some(Piece::Bishop) => is_bishop_semilegal(mv.src, mv.dst, b.all()),
                Some(Piece::Rook) => is_rook_semilegal(mv.src, mv.dst, b.all()),
                Some(Piece::Queen) => is_queen_semilegal(mv.src, mv.dst, b.all()),
                Some(Piece::Knight) => (attack::knight(mv.src) & dst).is_nonempty(),
                Some(Piece::King) => (attack::king(mv.src) & dst).is_nonempty(),
                Some(Piece::Pawn) | None => unreachable!(),
            }
        }
        MoveKind::CastlingKingside => {
            mv.src.rank() == geometry::castling_rank(c)
                && b.r.castling.has(c, CastlingSide::King)
                && (b.all() & castling::pass(c, CastlingSide::King)).is_empty()
                && !movegen::is_square_attacked(b, mv.src, c.inv())
                && !movegen::is_square_attacked(b, unsafe { mv.src.add_unchecked(1) }, c.inv())
        }
        MoveKind::CastlingQueenside => {
            mv.src.rank() == geometry::castling_rank(c)
                && b.r.castling.has(c, CastlingSide::Queen)
                && (b.all() & castling::pass(c, CastlingSide::Queen)).is_empty()
                && !movegen::is_square_attacked(b, mv.src, c.inv())
                && !movegen::is_square_attacked(b, unsafe { mv.src.add_unchecked(-1) }, c.inv())
        }
        _ => false,
    }
}

#[inline]
fn is_square_attacked_masked(
    b: &Board,
    s: Sq,
    c: Color,
    all: Bitboard,
    ours_mask: Bitboard,
) -> bool {
    let near = (b.piece(c, Piece::Pawn) & attack::pawn(c.inv(), s))
        | (b.piece(c, Piece::King) & attack::king(s))
        | (b.piece(c, Piece::Knight) & attack::knight(s));
    (near & ours_mask).is_nonempty()
        || (attack::bishop(s, all) & b.piece_diag(c) & ours_mask).is_nonempty()
        || (attack::rook(s, all) & b.piece_line(c) & ours_mask).is_nonempty()
}

#[inline(never)]
fn do_is_move_legal<C: generic::Color>(b: &Board, mv: Move) -> bool {
    let c = C::COLOR;
    let inv = c.inv();
    let src = Bitboard::one(mv.src);
    let dst = Bitboard::one(mv.dst);
    let src_cell = b.get(mv.src);

    if src_cell == Cell::make(c, Piece::King) {
        return !is_square_attacked_masked(b, mv.dst, inv, b.all() ^ src, Bitboard::FULL);
    }

    let king = b.king_pos(c);
    let all = (b.all() ^ src) | dst;
    let ours_mask = !dst;
    if mv.kind == MoveKind::Enpassant {
        let tmp = pawns::advance_forward(inv, dst);
        !is_square_attacked_masked(b, king, inv, all ^ tmp, ours_mask ^ tmp)
    } else {
        !is_square_attacked_masked(b, king, inv, all, ours_mask)
    }
}

enum UciMove {
    Null,
    Move {
        src: Sq,
        dst: Sq,
        promote: Option<Piece>,
    },
}

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum UciParseError {
    #[error("bad string length")]
    BadLength,
    #[error("bad source: {0}")]
    BadSrc(SqParseError),
    #[error("bad destination: {0}")]
    BadDst(SqParseError),
    #[error("bad promote char {0:?}")]
    BadPromote(char),
    #[error("invalid move: {0}")]
    Validate(#[from] ValidateError),
}

impl FromStr for UciMove {
    type Err = UciParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "0000" {
            return Ok(Self::Null);
        }
        if !matches!(s.len(), 4 | 5) {
            return Err(UciParseError::BadLength);
        }
        let src = Sq::from_str(&s[0..2]).map_err(UciParseError::BadSrc)?;
        let dst = Sq::from_str(&s[2..4]).map_err(UciParseError::BadDst)?;
        let promote = if s.len() == 5 {
            Some(match s.as_bytes()[4] {
                b'n' => Piece::Knight,
                b'b' => Piece::Bishop,
                b'r' => Piece::Rook,
                b'q' => Piece::Queen,
                b => return Err(UciParseError::BadPromote(b as char)),
            })
        } else {
            None
        };
        Ok(UciMove::Move { src, dst, promote })
    }
}

impl UciMove {
    fn into_move(self, b: &Board) -> Result<Move, ValidateError> {
        let c = b.r.side;
        match self {
            UciMove::Null => Ok(Move::NULL),
            UciMove::Move { src, dst, promote } => {
                let src_cell = b.get(src);
                if src_cell.color() != Some(c) {
                    return Err(ValidateError::NotWellFormed);
                }

                let kind = match promote {
                    Some(p) => MoveKind::promote_with(p).ok_or(ValidateError::NotWellFormed)?,
                    None => match src_cell.piece().unwrap() {
                        Piece::Pawn => {
                            if src.rank() == geometry::double_move_src_rank(c)
                                && dst.rank() == geometry::double_move_dst_rank(c)
                            {
                                MoveKind::PawnDouble
                            } else if src.file() != dst.file() && b.get(dst) == Cell::None {
                                MoveKind::Enpassant
                            } else {
                                MoveKind::PawnSimple
                            }
                        }
                        Piece::King => {
                            let r = geometry::castling_rank(c);
                            if src == Sq::make(File::E, r) && dst == Sq::make(File::G, r) {
                                MoveKind::CastlingKingside
                            } else if src == Sq::make(File::E, r) && dst == Sq::make(File::C, r) {
                                MoveKind::CastlingQueenside
                            } else {
                                MoveKind::Simple
                            }
                        }
                        _ => MoveKind::Simple,
                    },
                };

                Move::new(kind, src, dst)
            }
        }
    }
}

#[inline(never)]
fn do_diff_after_move<C: generic::Color>(
    b: &Board,
    mv: Move,
    u: &RawUndo,
    mut l: impl DiffListener,
) {
    let c = C::COLOR;
    let src_cell = b.get(mv.dst);
    match mv.kind {
        MoveKind::Simple | MoveKind::PawnSimple | MoveKind::PawnDouble => {
            l.del(mv.src, src_cell);
            l.upd(mv.dst, u.dst_cell, src_cell);
        }
        MoveKind::PromoteKnight
        | MoveKind::PromoteBishop
        | MoveKind::PromoteRook
        | MoveKind::PromoteQueen => {
            let pawn = Cell::make(c, Piece::Pawn);
            l.del(mv.src, pawn);
            l.upd(mv.dst, u.dst_cell, src_cell);
        }
        MoveKind::CastlingKingside => {
            let king = Cell::make(c, Piece::King);
            let rook = Cell::make(c, Piece::Rook);
            let rank = geometry::castling_rank(c);
            l.del(Sq::make(File::E, rank), king);
            l.add(Sq::make(File::F, rank), rook);
            l.add(Sq::make(File::G, rank), king);
            l.del(Sq::make(File::H, rank), rook);
        }
        MoveKind::CastlingQueenside => {
            let king = Cell::make(c, Piece::King);
            let rook = Cell::make(c, Piece::Rook);
            let rank = geometry::castling_rank(c);
            l.del(Sq::make(File::E, rank), king);
            l.add(Sq::make(File::D, rank), rook);
            l.add(Sq::make(File::C, rank), king);
            l.del(Sq::make(File::A, rank), rook);
        }
        MoveKind::Enpassant => {
            let tmp = unsafe { mv.dst.add_unchecked(-geometry::pawn_forward_delta(c)) };
            let our_pawn = Cell::make(c, Piece::Pawn);
            let their_pawn = Cell::make(c.inv(), Piece::Pawn);
            l.del(mv.src, our_pawn);
            l.del(tmp, their_pawn);
            l.add(mv.dst, our_pawn);
        }
        MoveKind::Null => {
            // Do nothing.
        }
    }
}

#[inline]
pub(crate) unsafe fn diff_after_move(b: &Board, mv: Move, u: &RawUndo, l: impl DiffListener) {
    match b.r.side {
        Color::White => do_diff_after_move::<generic::Black>(b, mv, u, l),
        Color::Black => do_diff_after_move::<generic::White>(b, mv, u, l),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use std::mem;

    #[test]
    fn test_size() {
        assert_eq!(mem::size_of::<Move>(), 4);
        assert_eq!(mem::size_of::<PackedMove>(), 2);
    }

    #[test]
    fn test_simple() {
        let mut b = Board::start();
        for (mv_str, fen_str, kind) in [
            (
                "e2e4",
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
                MoveKind::PawnDouble,
            ),
            (
                "b8c6",
                "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
                MoveKind::Simple,
            ),
            (
                "g1f3",
                "r1bqkbnr/pppppppp/2n5/8/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 2 2",
                MoveKind::Simple,
            ),
            (
                "e7e5",
                "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq e6 0 3",
                MoveKind::PawnDouble,
            ),
            (
                "f1b5",
                "r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 1 3",
                MoveKind::Simple,
            ),
            (
                "g8f6",
                "r1bqkb1r/pppp1ppp/2n2n2/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 2 4",
                MoveKind::Simple,
            ),
            (
                "e1g1",
                "r1bqkb1r/pppp1ppp/2n2n2/1B2p3/4P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 3 4",
                MoveKind::CastlingKingside,
            ),
            (
                "f6e4",
                "r1bqkb1r/pppp1ppp/2n5/1B2p3/4n3/5N2/PPPP1PPP/RNBQ1RK1 w kq - 0 5",
                MoveKind::Simple,
            ),
        ] {
            let m = Move::from_uci_legal(mv_str, &b).unwrap();
            assert_eq!(m.kind(), kind);
            let _ = unsafe { make_move_unchecked(&mut b, m) };
            assert_eq!(b.to_string(), fen_str);
            assert_eq!(b.raw().try_into(), Ok(b.clone()));
        }
    }

    #[test]
    fn test_promote() {
        let mut b = Board::from_str("1b1b1K2/2P5/8/8/7k/8/8/8 w - - 0 1").unwrap();
        let b_copy = b.clone();

        for (mv_str, fen_str) in [
            ("c7c8q", "1bQb1K2/8/8/8/7k/8/8/8 b - - 0 1"),
            ("c7b8n", "1N1b1K2/8/8/8/7k/8/8/8 b - - 0 1"),
            ("c7d8r", "1b1R1K2/8/8/8/7k/8/8/8 b - - 0 1"),
        ] {
            let m = Move::from_uci_legal(mv_str, &b).unwrap();
            let u = unsafe { make_move_unchecked(&mut b, m) };
            assert_eq!(b.to_string(), fen_str);
            assert_eq!(b.raw().try_into(), Ok(b.clone()));
            unsafe { unmake_move_unchecked(&mut b, m, u) };
            assert_eq!(b, b_copy);
        }
    }

    #[test]
    fn test_undo() {
        let mut b =
            Board::from_str("r1bqk2r/ppp2ppp/2np1n2/1Bb1p3/4P3/2PP1N2/PP3PPP/RNBQK2R w KQkq - 0 6")
                .unwrap();
        let b_copy = b.clone();

        for (mv_str, fen_str) in [
            (
                "e1g1",
                "r1bqk2r/ppp2ppp/2np1n2/1Bb1p3/4P3/2PP1N2/PP3PPP/RNBQ1RK1 b kq - 1 6",
            ),
            (
                "f3e5",
                "r1bqk2r/ppp2ppp/2np1n2/1Bb1N3/4P3/2PP4/PP3PPP/RNBQK2R b KQkq - 0 6",
            ),
            (
                "b2b4",
                "r1bqk2r/ppp2ppp/2np1n2/1Bb1p3/1P2P3/2PP1N2/P4PPP/RNBQK2R b KQkq b3 0 6",
            ),
            (
                "c3c4",
                "r1bqk2r/ppp2ppp/2np1n2/1Bb1p3/2P1P3/3P1N2/PP3PPP/RNBQK2R b KQkq - 0 6",
            ),
        ] {
            let m = Move::from_uci_legal(mv_str, &b).unwrap();
            let u = unsafe { make_move_unchecked(&mut b, m) };
            assert_eq!(b.to_string(), fen_str);
            assert_eq!(b.raw().try_into(), Ok(b.clone()));
            unsafe { unmake_move_unchecked(&mut b, m, u) };
            assert_eq!(b, b_copy);
        }
    }

    #[test]
    fn test_pawns() {
        let mut b = Board::from_str("3K4/3p4/8/3PpP2/8/5p2/6P1/2k5 w - e6 0 1").unwrap();
        let b_copy = b.clone();

        for (mv_str, fen_str) in [
            ("g2g3", "3K4/3p4/8/3PpP2/8/5pP1/8/2k5 b - - 0 1"),
            ("g2g4", "3K4/3p4/8/3PpP2/6P1/5p2/8/2k5 b - g3 0 1"),
            ("g2f3", "3K4/3p4/8/3PpP2/8/5P2/8/2k5 b - - 0 1"),
            ("d5e6", "3K4/3p4/4P3/5P2/8/5p2/6P1/2k5 b - - 0 1"),
            ("f5e6", "3K4/3p4/4P3/3P4/8/5p2/6P1/2k5 b - - 0 1"),
        ] {
            let m = Move::from_uci_legal(mv_str, &b).unwrap();
            let u = unsafe { make_move_unchecked(&mut b, m) };
            assert_eq!(b.to_string(), fen_str);
            assert_eq!(b.raw().try_into(), Ok(b.clone()));
            unsafe { unmake_move_unchecked(&mut b, m, u) };
            assert_eq!(b, b_copy);
        }
    }

    #[test]
    fn test_semi_legal() {
        let b =
            Board::from_str("r1bqk2r/ppp2ppp/2np1n2/1Bb1p3/4P3/2PP1N2/PP3PPP/RNBQK2R w KQkq - 0 6")
                .unwrap();

        let m = Move::from_uci("e1c1", &b).unwrap();
        assert!(!m.is_semilegal(&b));
        assert_eq!(m.semi_validate(&b), Err(ValidateError::NotSemiLegal));

        let m = Move::from_uci("b5e8", &b).unwrap();
        assert!(!m.is_semilegal(&b));
        assert_eq!(m.semi_validate(&b), Err(ValidateError::NotSemiLegal));

        assert_eq!(
            Move::from_uci("a3a4", &b),
            Err(UciParseError::Validate(ValidateError::NotWellFormed))
        );

        let m = Move::from_uci("e1d1", &b).unwrap();
        assert!(!m.is_semilegal(&b));
        assert_eq!(m.semi_validate(&b), Err(ValidateError::NotSemiLegal));

        assert_eq!(
            Move::from_uci("c3c5", &b),
            Err(UciParseError::Validate(ValidateError::NotWellFormed))
        );
    }

    #[test]
    fn test_pack() {
        let b = Board::start();

        let m = Move::from_uci_legal("e2e4", &b).unwrap();
        let p = PackedMove::from(m);
        assert_eq!(p.value(), 23844);
        let m2 = Move::from(p);
        assert_eq!(m, m2);

        let m = Move::from_uci_legal("g1f3", &b).unwrap();
        let p = PackedMove::from(m);
        assert_eq!(p.value(), 8109);
        let m2 = Move::from(p);
        assert_eq!(m, m2);
    }
}
