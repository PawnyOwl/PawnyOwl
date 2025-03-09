use crate::core::{Color, Rank};

#[inline]
pub const fn castling_rank(c: Color) -> Rank {
    match c {
        Color::White => Rank::R1,
        Color::Black => Rank::R8,
    }
}

#[inline]
pub const fn double_move_src_rank(c: Color) -> Rank {
    match c {
        Color::White => Rank::R2,
        Color::Black => Rank::R7,
    }
}

#[inline]
pub const fn double_move_dst_rank(c: Color) -> Rank {
    match c {
        Color::White => Rank::R4,
        Color::Black => Rank::R5,
    }
}

#[inline]
pub const fn promote_src_rank(c: Color) -> Rank {
    match c {
        Color::White => Rank::R7,
        Color::Black => Rank::R2,
    }
}

#[inline]
pub const fn promote_dst_rank(c: Color) -> Rank {
    match c {
        Color::White => Rank::R8,
        Color::Black => Rank::R1,
    }
}

#[inline]
pub const fn ep_src_rank(c: Color) -> Rank {
    match c {
        Color::White => Rank::R5,
        Color::Black => Rank::R4,
    }
}

#[inline]
pub const fn ep_dst_rank(c: Color) -> Rank {
    match c {
        Color::White => Rank::R6,
        Color::Black => Rank::R3,
    }
}

#[inline]
pub const fn pawn_forward_delta(c: Color) -> isize {
    match c {
        Color::White => -8,
        Color::Black => 8,
    }
}

#[inline]
pub const fn pawn_left_delta(c: Color) -> isize {
    match c {
        Color::White => -9,
        Color::Black => 7,
    }
}

#[inline]
pub const fn pawn_right_delta(c: Color) -> isize {
    match c {
        Color::White => -7,
        Color::Black => 9,
    }
}

pub mod bitboard {
    use crate::bitboard::Bitboard;
    use crate::core::{File, Rank};

    pub const DIAG: [Bitboard; 15] = [
        Bitboard::from_raw(0x0000000000000001),
        Bitboard::from_raw(0x0000000000000102),
        Bitboard::from_raw(0x0000000000010204),
        Bitboard::from_raw(0x0000000001020408),
        Bitboard::from_raw(0x0000000102040810),
        Bitboard::from_raw(0x0000010204081020),
        Bitboard::from_raw(0x0001020408102040),
        Bitboard::from_raw(0x0102040810204080),
        Bitboard::from_raw(0x0204081020408000),
        Bitboard::from_raw(0x0408102040800000),
        Bitboard::from_raw(0x0810204080000000),
        Bitboard::from_raw(0x1020408000000000),
        Bitboard::from_raw(0x2040800000000000),
        Bitboard::from_raw(0x4080000000000000),
        Bitboard::from_raw(0x8000000000000000),
    ];

    pub const ANTIDIAG: [Bitboard; 15] = [
        Bitboard::from_raw(0x0100000000000000),
        Bitboard::from_raw(0x0201000000000000),
        Bitboard::from_raw(0x0402010000000000),
        Bitboard::from_raw(0x0804020100000000),
        Bitboard::from_raw(0x1008040201000000),
        Bitboard::from_raw(0x2010080402010000),
        Bitboard::from_raw(0x4020100804020100),
        Bitboard::from_raw(0x8040201008040201),
        Bitboard::from_raw(0x0080402010080402),
        Bitboard::from_raw(0x0000804020100804),
        Bitboard::from_raw(0x0000008040201008),
        Bitboard::from_raw(0x0000000080402010),
        Bitboard::from_raw(0x0000000000804020),
        Bitboard::from_raw(0x0000000000008040),
        Bitboard::from_raw(0x0000000000000080),
    ];

    const RANK: [Bitboard; 8] = [
        Bitboard::from_raw(0x00000000000000ff),
        Bitboard::from_raw(0x000000000000ff00),
        Bitboard::from_raw(0x0000000000ff0000),
        Bitboard::from_raw(0x00000000ff000000),
        Bitboard::from_raw(0x000000ff00000000),
        Bitboard::from_raw(0x0000ff0000000000),
        Bitboard::from_raw(0x00ff000000000000),
        Bitboard::from_raw(0xff00000000000000),
    ];

    #[inline]
    pub const fn rank(r: Rank) -> Bitboard {
        RANK[r.index()]
    }

    const FILE: [Bitboard; 8] = [
        Bitboard::from_raw(0x0101010101010101),
        Bitboard::from_raw(0x0202020202020202),
        Bitboard::from_raw(0x0404040404040404),
        Bitboard::from_raw(0x0808080808080808),
        Bitboard::from_raw(0x1010101010101010),
        Bitboard::from_raw(0x2020202020202020),
        Bitboard::from_raw(0x4040404040404040),
        Bitboard::from_raw(0x8080808080808080),
    ];

    #[inline]
    pub const fn file(f: File) -> Bitboard {
        FILE[f.index()]
    }

    pub const LIGHT: Bitboard = Bitboard::from_raw(0xaa55aa55aa55aa55);
    pub const DARK: Bitboard = Bitboard::from_raw(0x55aa55aa55aa55aa);
}
