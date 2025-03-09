use crate::bitboard::Bitboard;
use crate::core::{self, CastlingRights, CastlingSide, Cell, Color, File, Piece, Rank, Sq};
use crate::moves::{self, Move, RawUndo};
use crate::{geometry, movegen, zobrist};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::num::ParseIntError;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RawBoard {
    pub squares: [Cell; 64],
    pub side: Color,
    pub castling: CastlingRights,
    pub ep_src: Option<Sq>,
    pub move_counter: u16,
    pub move_number: u16,
}

impl RawBoard {
    #[inline]
    pub const fn empty() -> Self {
        RawBoard {
            squares: [Cell::None; 64],
            side: Color::White,
            castling: CastlingRights::EMPTY,
            ep_src: None,
            move_counter: 0,
            move_number: 1,
        }
    }

    #[inline]
    pub fn start() -> Self {
        let mut res = RawBoard {
            squares: [Cell::None; 64],
            side: Color::White,
            castling: CastlingRights::FULL,
            ep_src: None,
            move_counter: 0,
            move_number: 1,
        };
        for file in File::iter() {
            res.put2(file, Rank::R2, Cell::WhitePawn);
            res.put2(file, Rank::R7, Cell::BlackPawn);
        }
        for (color, rank) in [(Color::White, Rank::R1), (Color::Black, Rank::R8)] {
            res.put2(File::A, rank, Cell::make(color, Piece::Rook));
            res.put2(File::B, rank, Cell::make(color, Piece::Knight));
            res.put2(File::C, rank, Cell::make(color, Piece::Bishop));
            res.put2(File::D, rank, Cell::make(color, Piece::Queen));
            res.put2(File::E, rank, Cell::make(color, Piece::King));
            res.put2(File::F, rank, Cell::make(color, Piece::Bishop));
            res.put2(File::G, rank, Cell::make(color, Piece::Knight));
            res.put2(File::H, rank, Cell::make(color, Piece::Rook));
        }
        res
    }

    #[inline]
    pub fn get(&self, s: Sq) -> Cell {
        unsafe { *self.squares.get_unchecked(s.index()) }
    }

    #[inline]
    pub fn get2(&self, file: File, rank: Rank) -> Cell {
        self.get(Sq::make(file, rank))
    }

    #[inline]
    pub fn put(&mut self, s: Sq, cell: Cell) {
        unsafe {
            *self.squares.get_unchecked_mut(s.index()) = cell;
        }
    }

    #[inline]
    pub fn put2(&mut self, file: File, rank: Rank, cell: Cell) {
        self.put(Sq::make(file, rank), cell);
    }

    #[inline]
    pub fn zobrist_hash(&self) -> u64 {
        let mut hash = if self.side == Color::White {
            zobrist::MOVE_SIDE
        } else {
            0
        };
        if let Some(p) = self.ep_src {
            hash ^= zobrist::enpassant(p);
        }
        hash ^= zobrist::castling(self.castling);
        for (i, cell) in self.squares.iter().enumerate() {
            if *cell != Cell::None {
                hash ^= zobrist::squares(*cell, Sq::from_index(i));
            }
        }
        hash
    }

    #[inline]
    pub fn ep_dst(&self) -> Option<Sq> {
        let p = self.ep_src?;
        Some(Sq::make(p.file(), geometry::ep_dst_rank(self.side)))
    }
}

impl Default for RawBoard {
    #[inline]
    fn default() -> RawBoard {
        RawBoard::empty()
    }
}

#[derive(Debug, Clone)]
pub struct Board {
    pub(crate) r: RawBoard,
    pub(crate) hash: u64,
    pub(crate) white: Bitboard,
    pub(crate) black: Bitboard,
    pub(crate) cells: [Bitboard; Cell::COUNT],
    pub(crate) all_v: Bitboard,
}

impl Board {
    pub fn start() -> Board {
        RawBoard::start().try_into().unwrap()
    }

    #[inline]
    pub fn raw(&self) -> &RawBoard {
        &self.r
    }

    #[inline]
    pub fn get(&self, s: Sq) -> Cell {
        self.r.get(s)
    }

    #[inline]
    pub fn get2(&self, file: File, rank: Rank) -> Cell {
        self.r.get2(file, rank)
    }

    #[inline]
    pub fn side(&self) -> Color {
        self.r.side
    }

    #[inline]
    pub fn color(&self, c: Color) -> Bitboard {
        if c == Color::White {
            self.white
        } else {
            self.black
        }
    }

    #[inline]
    pub(crate) fn color_mut(&mut self, c: Color) -> &mut Bitboard {
        if c == Color::White {
            &mut self.white
        } else {
            &mut self.black
        }
    }

    #[inline]
    pub fn cell(&self, c: Cell) -> Bitboard {
        unsafe { *self.cells.get_unchecked(c.index()) }
    }

    #[inline]
    pub fn piece(&self, c: Color, p: Piece) -> Bitboard {
        self.cell(Cell::make(c, p))
    }

    #[inline]
    pub fn piece_diag(&self, c: Color) -> Bitboard {
        self.piece(c, Piece::Bishop) | self.piece(c, Piece::Queen)
    }

    #[inline]
    pub fn piece_line(&self, c: Color) -> Bitboard {
        self.piece(c, Piece::Rook) | self.piece(c, Piece::Queen)
    }

    #[inline]
    pub(crate) fn cell_mut(&mut self, c: Cell) -> &mut Bitboard {
        unsafe { self.cells.get_unchecked_mut(c.index()) }
    }

    #[inline]
    pub fn king_pos(&self, c: Color) -> Sq {
        self.piece(c, Piece::King).into_iter().next().unwrap()
    }

    #[inline]
    pub fn zobrist_hash(&self) -> u64 {
        self.hash
    }

    #[inline]
    pub fn is_opponent_king_attacked(&self) -> bool {
        let c = self.r.side;
        movegen::is_square_attacked(self, self.king_pos(c.inv()), c)
    }

    #[inline]
    pub fn is_check(&self) -> bool {
        let c = self.r.side;
        movegen::is_square_attacked(self, self.king_pos(c), c.inv())
    }

    #[inline]
    pub fn checkers(&self) -> Bitboard {
        let c = self.r.side;
        movegen::square_attackers(self, self.king_pos(c), c.inv())
    }

    pub fn all(&self) -> Bitboard {
        // TODO: measure if storing all separately is worth it.
        self.all_v
    }

    #[inline]
    pub unsafe fn make_move_unchecked(&mut self, mv: Move) -> RawUndo {
        unsafe { moves::make_move_unchecked(self, mv) }
    }

    #[inline]
    pub unsafe fn unmake_move_unchecked(&mut self, mv: Move, u: RawUndo) {
        unsafe { moves::unmake_move_unchecked(self, mv, u) }
    }

    #[inline]
    pub fn make_move(&mut self, mv: Move) -> Result<(), moves::ValidateError> {
        mv.validate(self)?;
        _ = unsafe { self.make_move_unchecked(mv) };
        Ok(())
    }

    #[inline]
    pub fn make_uci_move(&mut self, mv: &str) -> Result<(), moves::UciParseError> {
        let mv = Move::from_uci_legal(mv, self)?;
        _ = unsafe { self.make_move_unchecked(mv) };
        Ok(())
    }
}

impl PartialEq for Board {
    #[inline]
    fn eq(&self, other: &Board) -> bool {
        self.r == other.r
    }
}

impl Eq for Board {}

impl Hash for Board {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.r.hash(state)
    }
}

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum ValidateError {
    #[error("bad enpassant position {0}")]
    BadEnpassant(Sq),
    #[error("too many pieces of color {0:?}")]
    TooManyPieces(Color),
    #[error("no king of color {0:?}")]
    NoKing(Color),
    #[error("more than one king of color {0:?}")]
    TooManyKings(Color),
    #[error("bad pawn position {0}")]
    BadPawn(Sq),
    #[error("opponent's king is attacked")]
    OpponentKingAttacked,
}

impl TryFrom<RawBoard> for Board {
    type Error = ValidateError;

    fn try_from(mut raw: RawBoard) -> Result<Board, ValidateError> {
        // Check enpassant
        if let Some(p) = raw.ep_src {
            // Check InvalidEnpassant
            if p.rank() != geometry::ep_src_rank(raw.side) {
                return Err(ValidateError::BadEnpassant(p));
            }

            // Reset enpassant if either there is no pawn or the cell on the pawn's path is occupied
            let pp = p.add(geometry::pawn_forward_delta(raw.side));
            if raw.get(p) != Cell::make(raw.side.inv(), Piece::Pawn) || raw.get(pp) != Cell::None {
                raw.ep_src = None;
            }
        }

        // Reset bad castling flags
        for color in [Color::White, Color::Black] {
            let rank = geometry::castling_rank(color);
            if raw.get2(File::E, rank) != Cell::make(color, Piece::King) {
                raw.castling.unset(color, CastlingSide::Queen);
                raw.castling.unset(color, CastlingSide::King);
            }
            if raw.get2(File::A, rank) != Cell::make(color, Piece::Rook) {
                raw.castling.unset(color, CastlingSide::Queen);
            }
            if raw.get2(File::H, rank) != Cell::make(color, Piece::Rook) {
                raw.castling.unset(color, CastlingSide::King);
            }
        }

        // Calculate bitboards
        let mut white = Bitboard::EMPTY;
        let mut black = Bitboard::EMPTY;
        let mut cells = [Bitboard::EMPTY; Cell::COUNT];
        for (idx, cell) in raw.squares.iter().enumerate() {
            let coord = Sq::from_index(idx);
            if let Some(color) = cell.color() {
                match color {
                    Color::White => white.set(coord),
                    Color::Black => black.set(coord),
                };
                cells[cell.index()].set(coord);
            }
        }

        // Check TooManyPieces, NoKing, TooManyKings
        if white.len() > 16 {
            return Err(ValidateError::TooManyPieces(Color::White));
        }
        if black.len() > 16 {
            return Err(ValidateError::TooManyPieces(Color::Black));
        }
        let white_king = cells[Cell::WhiteKing.index()];
        let black_king = cells[Cell::BlackKing.index()];
        if white_king.is_empty() {
            return Err(ValidateError::NoKing(Color::White));
        }
        if black_king.is_empty() {
            return Err(ValidateError::NoKing(Color::Black));
        }
        if white_king.len() > 1 {
            return Err(ValidateError::TooManyKings(Color::White));
        }
        if black_king.len() > 1 {
            return Err(ValidateError::TooManyKings(Color::Black));
        }

        // Check BadPawn
        let pawns = cells[Cell::WhitePawn.index()] | cells[Cell::BlackPawn.index()];
        const BAD_PAWN_POSES: Bitboard = Bitboard::from_raw(0xff000000000000ff);
        let bad_pawns = pawns & BAD_PAWN_POSES;
        if bad_pawns.is_nonempty() {
            return Err(ValidateError::BadPawn(
                bad_pawns.into_iter().next().unwrap(),
            ));
        }

        // Check OpponentKingAttacked
        let res = Board {
            r: raw,
            hash: raw.zobrist_hash(),
            white,
            black,
            cells,
            all_v: white | black,
        };
        if res.is_opponent_king_attacked() {
            return Err(ValidateError::OpponentKingAttacked);
        }

        Ok(res)
    }
}

impl TryFrom<&RawBoard> for Board {
    type Error = ValidateError;

    fn try_from(raw: &RawBoard) -> Result<Board, ValidateError> {
        (*raw).try_into()
    }
}

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum SquaresParseError {
    #[error("too many items in rank {0}")]
    RankOverflow(Rank),
    #[error("not enough items in rank {0}")]
    RankUnderflow(Rank),
    #[error("too many ranks")]
    Overflow,
    #[error("not enough ranks")]
    Underflow,
    #[error("unexpected char {0:?}")]
    UnexpectedChar(char),
}

fn parse_squares(s: &str) -> Result<[Cell; 64], SquaresParseError> {
    type Error = SquaresParseError;

    let mut file = 0_usize;
    let mut rank = 0_usize;
    let mut pos = 0_usize;
    let mut squares = [Cell::None; 64];
    for b in s.bytes() {
        match b {
            b'1'..=b'8' => {
                let add = (b - b'0') as usize;
                if file + add > 8 {
                    return Err(Error::RankOverflow(Rank::from_index(rank)));
                }
                file += add;
                pos += add;
            }
            b'/' => {
                if file < 8 {
                    return Err(Error::RankUnderflow(Rank::from_index(rank)));
                }
                rank += 1;
                file = 0;
                if rank >= 8 {
                    return Err(Error::Overflow);
                }
            }
            _ => {
                if file >= 8 {
                    return Err(Error::RankOverflow(Rank::from_index(rank)));
                }
                squares[pos] =
                    Cell::from_char(b as char).ok_or(Error::UnexpectedChar(b as char))?;
                file += 1;
                pos += 1;
            }
        };
    }

    if file < 8 {
        return Err(Error::RankUnderflow(Rank::from_index(rank)));
    }
    if rank < 7 {
        return Err(Error::Underflow);
    }
    assert_eq!(file, 8);
    assert_eq!(rank, 7);
    assert_eq!(pos, 64);

    Ok(squares)
}

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum RawFenParseError {
    #[error("non-ASCII data in FEN")]
    NonAscii,
    #[error("board not specified")]
    NoBoard,
    #[error("bad board: {0}")]
    Board(#[from] SquaresParseError),
    #[error("no move side")]
    NoMoveSide,
    #[error("bad move side: {0}")]
    MoveSide(#[from] core::ColorParseError),
    #[error("no castling rights")]
    NoCastling,
    #[error("bad castling rights: {0}")]
    Castling(#[from] core::CastlingRightsParseError),
    #[error("no enpassant")]
    NoEnpassant,
    #[error("bad enpassant: {0}")]
    Enpassant(#[from] core::SqParseError),
    #[error("bad enpassant rank {0}")]
    BadEnpassantRank(Rank),
    #[error("bad move counter: {0}")]
    MoveCounter(ParseIntError),
    #[error("bad move number: {0}")]
    MoveNumber(ParseIntError),
    #[error("extra data in FEN")]
    ExtraData,
}

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum FenParseError {
    #[error("cannot parse fen: {0}")]
    Fen(#[from] RawFenParseError),
    #[error("invalid position: {0}")]
    Valid(#[from] ValidateError),
}

fn parse_ep_src(s: &str, side: Color) -> Result<Option<Sq>, RawFenParseError> {
    if s == "-" {
        return Ok(None);
    }
    let ep = Sq::from_str(s)?;
    if ep.rank() != geometry::ep_dst_rank(side) {
        return Err(RawFenParseError::BadEnpassantRank(ep.rank()));
    }
    Ok(Some(Sq::make(ep.file(), geometry::ep_src_rank(side))))
}

impl FromStr for RawBoard {
    type Err = RawFenParseError;

    fn from_str(s: &str) -> Result<RawBoard, Self::Err> {
        type Error = RawFenParseError;

        if !s.is_ascii() {
            return Err(Error::NonAscii);
        }
        let mut iter = s.split(' ').fuse();

        let squares = parse_squares(iter.next().ok_or(Error::NoBoard)?)?;
        let side = Color::from_str(iter.next().ok_or(Error::NoMoveSide)?)?;
        let castling = CastlingRights::from_str(iter.next().ok_or(Error::NoCastling)?)?;
        let ep_src = parse_ep_src(iter.next().ok_or(Error::NoEnpassant)?, side)?;
        let move_counter = match iter.next() {
            Some(s) => u16::from_str(s).map_err(Error::MoveCounter)?,
            None => 0,
        };
        let move_number = match iter.next() {
            Some(s) => u16::from_str(s).map_err(Error::MoveNumber)?,
            None => 1,
        };

        if iter.next().is_some() {
            return Err(Error::ExtraData);
        }

        Ok(RawBoard {
            squares,
            side,
            castling,
            ep_src,
            move_counter,
            move_number,
        })
    }
}

impl FromStr for Board {
    type Err = FenParseError;

    fn from_str(s: &str) -> Result<Board, Self::Err> {
        Ok(RawBoard::from_str(s)?.try_into()?)
    }
}

fn format_squares(squares: &[Cell; 64], f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
    for rank in Rank::iter() {
        if rank.index() != 0 {
            write!(f, "/")?;
        }
        let mut empty = 0;
        for file in File::iter() {
            let cell = squares[Sq::make(file, rank).index()];
            if cell == Cell::None {
                empty += 1;
                continue;
            }
            if empty != 0 {
                write!(f, "{}", (b'0' + empty) as char)?;
                empty = 0;
            }
            write!(f, "{}", cell)?;
        }
        if empty != 0 {
            write!(f, "{}", (b'0' + empty) as char)?;
        }
    }
    Ok(())
}

impl fmt::Display for RawBoard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        format_squares(&self.squares, f)?;
        write!(f, " {} {}", self.side, self.castling)?;
        match self.ep_dst() {
            Some(p) => write!(f, " {}", p)?,
            None => write!(f, " -")?,
        };
        write!(f, " {} {}", self.move_counter, self.move_number)?;
        Ok(())
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.r.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_size() {
        assert_eq!(mem::size_of::<RawBoard>(), 72);
        assert_eq!(mem::size_of::<Board>(), 208);
    }

    #[test]
    fn test_start() {
        const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

        assert_eq!(RawBoard::start().to_string(), START_FEN);
        assert_eq!(Board::start().to_string(), START_FEN);
        assert_eq!(RawBoard::from_str(START_FEN), Ok(RawBoard::start()));
        assert_eq!(Board::from_str(START_FEN), Ok(Board::start()));
    }

    #[test]
    fn test_midgame() {
        const FEN: &str = "1rq1r1k1/1p3ppp/pB3n2/3ppP2/Pbb1P3/1PN2B2/2P2QPP/R1R4K w - - 1 21";

        let board = Board::from_str(FEN).unwrap();
        assert_eq!(board.to_string(), FEN);
        assert_eq!(board.get2(File::B, Rank::R4), Cell::BlackBishop,);
        assert_eq!(board.get2(File::F, Rank::R2), Cell::WhiteQueen,);
        assert_eq!(board.king_pos(Color::White), Sq::make(File::H, Rank::R1));
        assert_eq!(board.king_pos(Color::Black), Sq::make(File::G, Rank::R8));
        assert_eq!(board.raw().side, Color::White);
        assert_eq!(board.raw().castling, CastlingRights::EMPTY);
        assert_eq!(board.raw().ep_src, None);
        assert_eq!(board.raw().move_counter, 1);
        assert_eq!(board.raw().move_number, 21);
    }

    #[test]
    fn test_fixes_on_validate() {
        const FEN: &str = "r1bq1b1r/ppppkppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK1R1 w KQkq c6 6 5";

        let raw = RawBoard::from_str(FEN).unwrap();
        assert_eq!(raw.castling, CastlingRights::FULL);
        assert_eq!(raw.ep_src, Some(Sq::make(File::C, Rank::R5)));
        assert_eq!(raw.ep_dst(), Some(Sq::make(File::C, Rank::R6)));
        assert_eq!(raw.to_string(), FEN);

        let board: Board = raw.try_into().unwrap();
        assert_eq!(
            board.raw().castling,
            CastlingRights::EMPTY.with(Color::White, CastlingSide::Queen)
        );
        assert_eq!(board.raw().ep_src, None);
        assert_eq!(board.raw().ep_dst(), None);
        assert_eq!(
            board.to_string(),
            "r1bq1b1r/ppppkppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK1R1 w Q - 6 5"
        );
    }

    #[test]
    fn test_incomplete() {
        assert_eq!(
            RawBoard::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR"),
            Err(RawFenParseError::NoMoveSide)
        );

        assert_eq!(
            RawBoard::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w"),
            Err(RawFenParseError::NoCastling)
        );

        assert_eq!(
            RawBoard::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq"),
            Err(RawFenParseError::NoEnpassant)
        );

        let raw =
            RawBoard::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -").unwrap();
        assert_eq!(raw.move_counter, 0);
        assert_eq!(raw.move_number, 1);

        let raw =
            RawBoard::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 10").unwrap();
        assert_eq!(raw.move_counter, 10);
        assert_eq!(raw.move_number, 1);
    }
}
