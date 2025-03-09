use std::{fmt, hint, str::FromStr};
use thiserror::Error;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(u8)]
pub enum File {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    E = 4,
    F = 5,
    G = 6,
    H = 7,
}

impl File {
    #[inline]
    pub const fn index(self) -> usize {
        self as u8 as usize
    }

    #[inline]
    pub const unsafe fn from_index_unchecked(val: usize) -> Self {
        match val {
            0 => File::A,
            1 => File::B,
            2 => File::C,
            3 => File::D,
            4 => File::E,
            5 => File::F,
            6 => File::G,
            7 => File::H,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    #[inline]
    pub const fn from_index(val: usize) -> Self {
        assert!(val < 8, "file index must be between 0 and 7");
        unsafe { Self::from_index_unchecked(val) }
    }

    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..8).map(|x| unsafe { Self::from_index_unchecked(x) })
    }

    #[inline]
    unsafe fn from_char_unchecked(c: char) -> Self {
        unsafe { File::from_index_unchecked((u32::from(c) - u32::from('a')) as usize) }
    }

    #[inline]
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'a'..='h' => Some(unsafe { Self::from_char_unchecked(c) }),
            _ => None,
        }
    }

    #[inline]
    pub fn as_char(self) -> char {
        (b'a' + self as u8) as char
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_char())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(u8)]
pub enum Rank {
    R8 = 0,
    R7 = 1,
    R6 = 2,
    R5 = 3,
    R4 = 4,
    R3 = 5,
    R2 = 6,
    R1 = 7,
}

impl Rank {
    #[inline]
    pub const fn index(self) -> usize {
        self as u8 as usize
    }

    #[inline]
    pub const unsafe fn from_index_unchecked(val: usize) -> Self {
        match val {
            0 => Rank::R8,
            1 => Rank::R7,
            2 => Rank::R6,
            3 => Rank::R5,
            4 => Rank::R4,
            5 => Rank::R3,
            6 => Rank::R2,
            7 => Rank::R1,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    #[inline]
    pub const fn from_index(val: usize) -> Self {
        assert!(val < 8, "rank index must be between 0 and 7");
        unsafe { Self::from_index_unchecked(val) }
    }

    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..8).map(|x| unsafe { Self::from_index_unchecked(x) })
    }

    #[inline]
    unsafe fn from_char_unchecked(c: char) -> Self {
        unsafe { Rank::from_index_unchecked((u32::from('8') - u32::from(c)) as usize) }
    }

    #[inline]
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '1'..='8' => Some(unsafe { Self::from_char_unchecked(c) }),
            _ => None,
        }
    }

    #[inline]
    pub fn as_char(self) -> char {
        (b'8' - self as u8) as char
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_char())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Sq(u8);

impl Sq {
    #[inline]
    pub const fn from_index(val: usize) -> Sq {
        assert!(val < 64, "square must be between 0 and 63");
        Sq(val as u8)
    }

    #[inline]
    pub const unsafe fn from_index_unchecked(val: usize) -> Sq {
        Sq(val as u8)
    }

    #[inline]
    pub const fn make(file: File, rank: Rank) -> Sq {
        Sq(((rank as u8) << 3) | file as u8)
    }

    #[inline]
    pub const fn file(self) -> File {
        unsafe { File::from_index_unchecked((self.0 & 7) as usize) }
    }

    #[inline]
    pub const fn rank(self) -> Rank {
        unsafe { Rank::from_index_unchecked((self.0 >> 3) as usize) }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub const fn flipped_rank(self) -> Sq {
        Sq(self.0 ^ 56)
    }

    #[inline]
    pub const fn flipped_file(self) -> Sq {
        Sq(self.0 ^ 7)
    }

    #[inline]
    pub const fn diag(self) -> usize {
        self.file().index() + self.rank().index()
    }

    #[inline]
    pub const fn antidiag(self) -> usize {
        7 - self.rank().index() + self.file().index()
    }

    #[inline]
    pub const fn add(self, delta: isize) -> Sq {
        Sq::from_index(self.index().wrapping_add(delta as usize))
    }

    #[inline]
    pub const unsafe fn add_unchecked(self, delta: isize) -> Sq {
        unsafe { Sq::from_index_unchecked(self.index().wrapping_add(delta as usize)) }
    }

    #[inline]
    pub fn shift(self, delta_file: isize, delta_rank: isize) -> Option<Sq> {
        let new_file = self.file().index().wrapping_add(delta_file as usize);
        let new_rank = self.rank().index().wrapping_add(delta_rank as usize);
        if new_file >= 8 || new_rank >= 8 {
            return None;
        }
        unsafe {
            Some(Sq::make(
                File::from_index_unchecked(new_file),
                Rank::from_index_unchecked(new_rank),
            ))
        }
    }

    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        (0_u8..64_u8).map(Sq)
    }
}

impl fmt::Debug for Sq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        if self.0 < 64 {
            return write!(f, "Sq({})", self);
        }
        write!(f, "Sq(?{:?})", self.0)
    }
}

impl fmt::Display for Sq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}{}", self.file().as_char(), self.rank().as_char())
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SqParseError {
    #[error("bad file char {0:?}")]
    BadFileChar(char),
    #[error("bad rank char {0:?}")]
    BadRankChar(char),
    #[error("bad length")]
    BadLength,
}

impl FromStr for Sq {
    type Err = SqParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 2 {
            return Err(SqParseError::BadLength);
        }
        let bytes = s.as_bytes();
        let (file_ch, rank_ch) = (bytes[0] as char, bytes[1] as char);
        Ok(Sq::make(
            File::from_char(file_ch).ok_or(SqParseError::BadFileChar(file_ch))?,
            Rank::from_char(rank_ch).ok_or(SqParseError::BadRankChar(rank_ch))?,
        ))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    #[inline]
    pub const fn inv(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    #[inline]
    pub fn as_char(self) -> char {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
    }

    #[inline]
    pub fn from_char(c: char) -> Option<Color> {
        match c {
            'w' => Some(Color::White),
            'b' => Some(Color::Black),
            _ => None,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_char())
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ColorParseError {
    #[error("bad color char {0:?}")]
    BadChar(char),
    #[error("bad string length")]
    BadLength,
}

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 1 {
            return Err(ColorParseError::BadLength);
        }
        let ch = s.as_bytes()[0] as char;
        Color::from_char(s.as_bytes()[0] as char).ok_or(ColorParseError::BadChar(ch))
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Piece {
    Pawn = 0,
    King = 1,
    Knight = 2,
    Bishop = 3,
    Rook = 4,
    Queen = 5,
}

impl Piece {
    pub const COUNT: usize = 6;

    #[inline]
    pub const fn index(self) -> usize {
        self as u8 as usize
    }

    #[inline]
    pub const unsafe fn from_index_unchecked(val: usize) -> Self {
        match val {
            0 => Self::Pawn,
            1 => Self::King,
            2 => Self::Knight,
            3 => Self::Bishop,
            4 => Self::Rook,
            5 => Self::Queen,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    #[inline]
    pub const fn from_index(val: usize) -> Self {
        assert!(val < Self::COUNT, "index too large");
        unsafe { Self::from_index_unchecked(val) }
    }

    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..Self::COUNT).map(|x| unsafe { Self::from_index_unchecked(x) })
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cell {
    #[default]
    None = 0,
    WhitePawn = 1,
    WhiteKing = 2,
    WhiteKnight = 3,
    WhiteBishop = 4,
    WhiteRook = 5,
    WhiteQueen = 6,
    BlackPawn = 7,
    BlackKing = 8,
    BlackKnight = 9,
    BlackBishop = 10,
    BlackRook = 11,
    BlackQueen = 12,
}

impl Cell {
    pub const COUNT: usize = 13;

    #[inline]
    pub const unsafe fn from_index_unchecked(val: usize) -> Cell {
        match val {
            0 => Self::None,
            1 => Self::WhitePawn,
            2 => Self::WhiteKing,
            3 => Self::WhiteKnight,
            4 => Self::WhiteBishop,
            5 => Self::WhiteRook,
            6 => Self::WhiteQueen,
            7 => Self::BlackPawn,
            8 => Self::BlackKing,
            9 => Self::BlackKnight,
            10 => Self::BlackBishop,
            11 => Self::BlackRook,
            12 => Self::BlackQueen,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    #[inline]
    pub const fn from_index(val: usize) -> Cell {
        assert!(val < Self::COUNT, "index too large");
        unsafe { Self::from_index_unchecked(val) }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self as u8 as usize
    }

    #[inline]
    pub const fn make(c: Color, p: Piece) -> Cell {
        unsafe {
            match c {
                Color::White => Self::from_index_unchecked(1 + p.index()),
                Color::Black => Self::from_index_unchecked(7 + p.index()),
            }
        }
    }

    #[inline]
    pub const fn color(self) -> Option<Color> {
        match self {
            Cell::None => None,
            Cell::WhitePawn
            | Cell::WhiteKing
            | Cell::WhiteKnight
            | Cell::WhiteBishop
            | Cell::WhiteRook
            | Cell::WhiteQueen => Some(Color::White),
            Cell::BlackPawn
            | Cell::BlackKing
            | Cell::BlackKnight
            | Cell::BlackBishop
            | Cell::BlackRook
            | Cell::BlackQueen => Some(Color::Black),
        }
    }

    #[inline]
    pub const fn piece(self) -> Option<Piece> {
        match self {
            Cell::None => None,
            Cell::WhitePawn | Cell::BlackPawn => Some(Piece::Pawn),
            Cell::WhiteKing | Cell::BlackKing => Some(Piece::King),
            Cell::WhiteKnight | Cell::BlackKnight => Some(Piece::Knight),
            Cell::WhiteBishop | Cell::BlackBishop => Some(Piece::Bishop),
            Cell::WhiteRook | Cell::BlackRook => Some(Piece::Rook),
            Cell::WhiteQueen | Cell::BlackQueen => Some(Piece::Queen),
        }
    }

    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..Self::COUNT).map(|x| unsafe { Self::from_index_unchecked(x) })
    }

    #[inline]
    pub fn as_char(self) -> char {
        b".PKNBRQpknbrq"[self.index()] as char
    }

    #[inline]
    pub fn from_char(c: char) -> Option<Self> {
        if c == '.' {
            return Some(Cell::None);
        }
        let color = if c.is_ascii_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        let piece = match c.to_ascii_lowercase() {
            'p' => Piece::Pawn,
            'k' => Piece::King,
            'n' => Piece::Knight,
            'b' => Piece::Bishop,
            'r' => Piece::Rook,
            'q' => Piece::Queen,
            _ => return None,
        };
        Some(Cell::make(color, piece))
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_char())
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CellParseError {
    #[error("bad cell char {0:?}")]
    BadChar(char),
    #[error("bad string length")]
    BadLength,
}

impl FromStr for Cell {
    type Err = CellParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 1 {
            return Err(CellParseError::BadLength);
        }
        let ch = s.as_bytes()[0] as char;
        Cell::from_char(s.as_bytes()[0] as char).ok_or(CellParseError::BadChar(ch))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CastlingSide {
    Queen = 0,
    King = 1,
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CastlingRights(u8);

impl CastlingRights {
    #[inline]
    const fn to_index(c: Color, s: CastlingSide) -> u8 {
        ((c as u8) << 1) | s as u8
    }

    #[inline]
    const fn to_color_mask(c: Color) -> u8 {
        3 << ((c as u8) << 1)
    }

    pub const EMPTY: CastlingRights = CastlingRights(0);
    pub const FULL: CastlingRights = CastlingRights(15);

    #[inline]
    pub const fn has(self, c: Color, s: CastlingSide) -> bool {
        ((self.0 >> Self::to_index(c, s)) & 1) != 0
    }

    #[inline]
    pub const fn has_color(self, c: Color) -> bool {
        (self.0 & Self::to_color_mask(c)) != 0
    }

    #[inline]
    pub const fn with(self, c: Color, s: CastlingSide) -> CastlingRights {
        CastlingRights(self.0 | (1_u8 << Self::to_index(c, s)))
    }

    #[inline]
    pub const fn without(self, c: Color, s: CastlingSide) -> CastlingRights {
        CastlingRights(self.0 & !(1_u8 << Self::to_index(c, s)))
    }

    #[inline]
    pub fn set(&mut self, c: Color, s: CastlingSide) {
        *self = self.with(c, s)
    }

    #[inline]
    pub fn unset(&mut self, c: Color, s: CastlingSide) {
        *self = self.without(c, s)
    }

    #[inline]
    pub fn unset_color(&mut self, c: Color) {
        self.unset(c, CastlingSide::King);
        self.unset(c, CastlingSide::Queen);
    }

    #[inline]
    pub const fn from_index(val: usize) -> CastlingRights {
        assert!(val < 16, "raw castling rights must be between 0 and 15");
        CastlingRights(val as u8)
    }

    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Debug for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        if self.0 < 16 {
            return write!(f, "CastlingRights({})", self);
        }
        write!(f, "CastlingRights(?{:?})", self.0)
    }
}

impl fmt::Display for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        if *self == Self::EMPTY {
            return write!(f, "-");
        }
        if self.has(Color::White, CastlingSide::King) {
            write!(f, "K")?;
        }
        if self.has(Color::White, CastlingSide::Queen) {
            write!(f, "Q")?;
        }
        if self.has(Color::Black, CastlingSide::King) {
            write!(f, "k")?;
        }
        if self.has(Color::Black, CastlingSide::Queen) {
            write!(f, "q")?;
        }
        Ok(())
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CastlingRightsParseError {
    #[error("bad castling char {0:?}")]
    BadChar(char),
    #[error("duplicate castling char {0:?}")]
    DuplicateChar(char),
    #[error("the string is empty")]
    EmptyString,
}

impl FromStr for CastlingRights {
    type Err = CastlingRightsParseError;

    fn from_str(s: &str) -> Result<CastlingRights, Self::Err> {
        type Error = CastlingRightsParseError;
        if s == "-" {
            return Ok(CastlingRights::EMPTY);
        }
        if s.is_empty() {
            return Err(Error::EmptyString);
        }
        let mut res = CastlingRights::EMPTY;
        for b in s.bytes() {
            let (color, side) = match b {
                b'K' => (Color::White, CastlingSide::King),
                b'Q' => (Color::White, CastlingSide::Queen),
                b'k' => (Color::Black, CastlingSide::King),
                b'q' => (Color::Black, CastlingSide::Queen),
                _ => return Err(Error::BadChar(b as char)),
            };
            if res.has(color, side) {
                return Err(Error::DuplicateChar(b as char));
            }
            res.set(color, side);
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file() {
        for (idx, file) in File::iter().enumerate() {
            assert_eq!(file.index(), idx);
            assert_eq!(File::from_index(idx), file);
        }
    }

    #[test]
    fn test_rank() {
        for (idx, rank) in Rank::iter().enumerate() {
            assert_eq!(rank.index(), idx);
            assert_eq!(Rank::from_index(idx), rank);
        }
    }

    #[test]
    fn test_piece() {
        for (idx, piece) in Piece::iter().enumerate() {
            assert_eq!(piece.index(), idx);
            assert_eq!(Piece::from_index(idx), piece);
        }
    }

    #[test]
    fn test_sq() {
        let mut sqs = Vec::new();
        for rank in Rank::iter() {
            for file in File::iter() {
                let sq = Sq::make(file, rank);
                assert_eq!(sq.file(), file);
                assert_eq!(sq.rank(), rank);
                sqs.push(sq);
            }
        }
        assert_eq!(sqs, Sq::iter().collect::<Vec<_>>());
    }

    #[test]
    fn test_cell() {
        assert_eq!(Cell::None.color(), None);
        assert_eq!(Cell::None.piece(), None);
        let mut cells = vec![Cell::None];
        for color in [Color::White, Color::Black] {
            for piece in [
                Piece::Pawn,
                Piece::King,
                Piece::Knight,
                Piece::Bishop,
                Piece::Rook,
                Piece::Queen,
            ] {
                let cell = Cell::make(color, piece);
                assert_eq!(cell.color(), Some(color));
                assert_eq!(cell.piece(), Some(piece));
                cells.push(cell);
            }
        }
        assert_eq!(cells, Cell::iter().collect::<Vec<_>>());
    }

    #[test]
    fn test_castling() {
        let empty = CastlingRights::EMPTY;
        assert!(!empty.has(Color::White, CastlingSide::Queen));
        assert!(!empty.has(Color::White, CastlingSide::King));
        assert!(!empty.has_color(Color::White));
        assert!(!empty.has(Color::Black, CastlingSide::Queen));
        assert!(!empty.has(Color::Black, CastlingSide::King));
        assert!(!empty.has_color(Color::Black));
        assert_eq!(empty.to_string(), "-");
        assert_eq!(CastlingRights::from_str("-"), Ok(empty));

        let full = CastlingRights::FULL;
        assert!(full.has(Color::White, CastlingSide::Queen));
        assert!(full.has(Color::White, CastlingSide::King));
        assert!(full.has_color(Color::White));
        assert!(full.has(Color::Black, CastlingSide::Queen));
        assert!(full.has(Color::Black, CastlingSide::King));
        assert!(full.has_color(Color::Black));
        assert_eq!(full.to_string(), "KQkq");
        assert_eq!(CastlingRights::from_str("KQkq"), Ok(full));

        let mut rights = CastlingRights::EMPTY;
        rights.set(Color::White, CastlingSide::King);
        assert!(!rights.has(Color::White, CastlingSide::Queen));
        assert!(rights.has(Color::White, CastlingSide::King));
        assert!(rights.has_color(Color::White));
        assert!(!rights.has(Color::Black, CastlingSide::Queen));
        assert!(!rights.has(Color::Black, CastlingSide::King));
        assert!(!rights.has_color(Color::Black));
        assert_eq!(rights.to_string(), "K");
        assert_eq!(CastlingRights::from_str("K"), Ok(rights));

        rights.unset(Color::White, CastlingSide::King);
        rights.set(Color::Black, CastlingSide::Queen);
        assert!(!rights.has(Color::White, CastlingSide::Queen));
        assert!(!rights.has(Color::White, CastlingSide::King));
        assert!(!rights.has_color(Color::White));
        assert!(rights.has(Color::Black, CastlingSide::Queen));
        assert!(!rights.has(Color::Black, CastlingSide::King));
        assert!(rights.has_color(Color::Black));
        assert_eq!(rights.to_string(), "q");
        assert_eq!(CastlingRights::from_str("q"), Ok(rights));
    }

    #[test]
    fn test_sq_str() {
        assert_eq!(Sq::make(File::B, Rank::R4).to_string(), "b4".to_string());
        assert_eq!(Sq::make(File::A, Rank::R1).to_string(), "a1".to_string());
        assert_eq!(Sq::from_str("a1"), Ok(Sq::make(File::A, Rank::R1)));
        assert_eq!(Sq::from_str("b4"), Ok(Sq::make(File::B, Rank::R4)));
        assert!(Sq::from_str("h9").is_err());
        assert!(Sq::from_str("i4").is_err());
    }

    #[test]
    fn test_cell_str() {
        for cell in Cell::iter() {
            let s = cell.to_string();
            assert_eq!(Cell::from_str(&s), Ok(cell));
        }
    }
}
