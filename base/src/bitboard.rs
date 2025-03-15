use crate::core::{File, Rank, Sq};
use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};
use std::fmt;

#[derive(
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
    BitXor,
    BitXorAssign,
    Not,
)]
pub struct Bitboard(u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const FULL: Bitboard = Bitboard(u64::MAX);

    #[inline]
    pub const fn one(sq: Sq) -> Bitboard {
        Bitboard(1_u64 << sq.index())
    }

    #[inline]
    pub const fn with(self, sq: Sq) -> Bitboard {
        Bitboard(self.0 | (1_u64 << sq.index()))
    }

    #[inline]
    pub const fn with2(self, file: File, rank: Rank) -> Bitboard {
        self.with(Sq::make(file, rank))
    }

    #[inline]
    pub const fn without(self, sq: Sq) -> Bitboard {
        Bitboard(self.0 & !(1_u64 << sq.index()))
    }

    #[inline]
    pub const fn without2(self, file: File, rank: Rank) -> Bitboard {
        self.without(Sq::make(file, rank))
    }

    #[inline]
    pub const fn shl(self, by: usize) -> Bitboard {
        Bitboard(self.0 << by)
    }

    #[inline]
    pub const fn shr(self, by: usize) -> Bitboard {
        Bitboard(self.0 >> by)
    }

    #[inline]
    pub fn deposit_bits(self, mut x: u64) -> Bitboard {
        let mut res: u64 = 0;
        let mut msk = self.0;
        while msk != 0 {
            let bit = msk & msk.wrapping_neg();
            if (x & 1) != 0 {
                res |= bit;
            }
            msk ^= bit;
            x >>= 1;
        }
        Bitboard(res)
    }

    #[inline]
    pub fn set(&mut self, sq: Sq) {
        *self = self.with(sq);
    }

    #[inline]
    pub fn unset(&mut self, sq: Sq) {
        *self = self.without(sq);
    }

    #[inline]
    pub const fn has(self, sq: Sq) -> bool {
        ((self.0 >> sq.index()) & 1) != 0
    }

    #[inline]
    pub const fn has2(self, file: File, rank: Rank) -> bool {
        self.has(Sq::make(file, rank))
    }

    #[inline]
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_nonempty(self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub const fn from_raw(val: u64) -> Bitboard {
        Bitboard(val)
    }

    #[inline]
    pub const fn as_raw(self) -> u64 {
        self.0
    }
}

impl From<Bitboard> for u64 {
    #[inline]
    fn from(b: Bitboard) -> u64 {
        b.0
    }
}

impl From<u64> for Bitboard {
    #[inline]
    fn from(u: u64) -> Bitboard {
        Bitboard(u)
    }
}

impl fmt::Debug for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "Bitboard({})", self)
    }
}

impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let v = self.0.reverse_bits();
        write!(
            f,
            "{:08b}/{:08b}/{:08b}/{:08b}/{:08b}/{:08b}/{:08b}/{:08b}",
            (v >> 56) & 0xff,
            (v >> 48) & 0xff,
            (v >> 40) & 0xff,
            (v >> 32) & 0xff,
            (v >> 24) & 0xff,
            (v >> 16) & 0xff,
            (v >> 8) & 0xff,
            v & 0xff,
        )
    }
}

#[derive(Clone)]
pub struct Iter(u64);

impl Iterator for Iter {
    type Item = Sq;

    #[inline]
    fn next(&mut self) -> Option<Sq> {
        if self.0 == 0 {
            return None;
        }
        let bit = self.0.trailing_zeros();
        self.0 &= self.0.wrapping_sub(1_u64);
        unsafe { Some(Sq::from_index_unchecked(bit as usize)) }
    }
}

impl IntoIterator for Bitboard {
    type Item = Sq;
    type IntoIter = Iter;

    #[inline]
    fn into_iter(self) -> Iter {
        Iter(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{File, Rank, Sq};

    #[test]
    fn test_iter() {
        let bb = Bitboard::EMPTY
            .with(Sq::make(File::A, Rank::R4))
            .with(Sq::make(File::E, Rank::R2))
            .with(Sq::make(File::F, Rank::R3));
        assert_eq!(
            bb.into_iter().collect::<Vec<_>>(),
            vec![
                Sq::make(File::A, Rank::R4),
                Sq::make(File::F, Rank::R3),
                Sq::make(File::E, Rank::R2)
            ],
        );
    }

    #[test]
    fn test_bitops() {
        let ca = Sq::make(File::A, Rank::R4);
        let cb = Sq::make(File::E, Rank::R2);
        let cc = Sq::make(File::F, Rank::R3);

        let bb1 = Bitboard::EMPTY.with(ca).with(cb);
        let bb2 = Bitboard::EMPTY.with(cb).with(cc);
        assert_eq!(bb1 & bb2, Bitboard::EMPTY.with(cb));
        assert_eq!(bb1 | bb2, Bitboard::EMPTY.with(ca).with(cb).with(cc));
        assert_eq!(bb1 ^ bb2, Bitboard::EMPTY.with(ca).with(cc));

        assert_eq!((!bb1).into_iter().count(), 62);
        assert_eq!((!bb1).len(), 62);
    }

    #[test]
    fn test_format() {
        let bb = Bitboard::EMPTY
            .with(Sq::make(File::A, Rank::R4))
            .with(Sq::make(File::E, Rank::R2))
            .with(Sq::make(File::F, Rank::R3))
            .with(Sq::make(File::H, Rank::R8));
        assert_eq!(
            bb.to_string(),
            "00000001/00000000/00000000/00000000/10000000/00000100/00001000/00000000"
        );
    }
}
