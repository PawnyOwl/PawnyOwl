use std::cmp::Ordering;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Bound {
    Lower,
    Upper,
    Exact,
}

impl Default for Bound {
    #[inline]
    fn default() -> Self {
        Bound::Exact
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Score {
    Cp(i32),
    Mate { moves: u32, win: bool },
}

impl Default for Score {
    #[inline]
    fn default() -> Self {
        Score::Cp(0)
    }
}

impl Score {
    #[inline]
    pub fn inv(self) -> Self {
        match self {
            Self::Cp(x) => Self::Cp(-x),
            Self::Mate { moves, win } => Self::Mate { moves, win: !win },
        }
    }

    fn as_cmp_tuple(&self) -> (i32, i64) {
        match *self {
            Self::Cp(val) => (0, val as i64),
            Self::Mate { moves, win: true } => (1, -(moves as i64)),
            Self::Mate { moves, win: false } => (-1, moves as i64),
        }
    }
}

impl PartialOrd for Score {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Score {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_cmp_tuple().cmp(&other.as_cmp_tuple())
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug, Hash)]
pub struct BoundedScore {
    pub score: Score,
    pub bound: Bound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_rel() {
        let mut src = [
            Score::Cp(-100),
            Score::Cp(280),
            Score::Cp(0),
            Score::Cp(-410),
            Score::Mate {
                moves: 2,
                win: true,
            },
            Score::Mate {
                moves: 0,
                win: true,
            },
            Score::Mate {
                moves: 5,
                win: true,
            },
            Score::Mate {
                moves: 3,
                win: false,
            },
            Score::Mate {
                moves: 0,
                win: false,
            },
            Score::Mate {
                moves: 9,
                win: false,
            },
        ];
        let res = [
            Score::Mate {
                moves: 0,
                win: false,
            },
            Score::Mate {
                moves: 3,
                win: false,
            },
            Score::Mate {
                moves: 9,
                win: false,
            },
            Score::Cp(-410),
            Score::Cp(-100),
            Score::Cp(0),
            Score::Cp(280),
            Score::Mate {
                moves: 5,
                win: true,
            },
            Score::Mate {
                moves: 2,
                win: true,
            },
            Score::Mate {
                moves: 0,
                win: true,
            },
        ];
        src.sort();
        assert_eq!(src, res);
    }
}
