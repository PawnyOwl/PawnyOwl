use std::ops::Mul;
use derive_more::{Add, AddAssign, Sub, SubAssign};

pub type Stage = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Add, AddAssign, Sub, SubAssign)]
pub struct Score(i16);

impl Score {
    #[inline]
    pub fn new(v: i16) -> Self {
        Score(v)
    }

    #[inline]
    pub fn mate(move_count: usize) -> Self {
        Self::min() + Score(1 + move_count as i16)
    }

    #[inline]
    pub fn max() -> Self {
        Score(30000)
    }

    #[inline]
    pub fn min() -> Self {
        Score(-30000)
    }

    #[inline]
    pub fn mate_bound() -> Self {
        Score(-25000)
    }

    #[inline]
    pub fn value(self) -> i16 {
        self.0
    }
}

impl Mul<i16> for Score {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: i16) -> Self::Output {
        Score(self.0 * scalar)
    }
}

impl From<Score> for i32 {
    #[inline]
    fn from(score: Score) -> i32 {
        score.0 as i32
    }
}

impl From<i32> for Score {
    #[inline]
    fn from(val: i32) -> Score {
        Score::new(val as i16)
    }
}
