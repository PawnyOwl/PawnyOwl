use pawnyowl_board::{Board, Color, MoveGen, MoveList};
use std::str::FromStr;

const HPERFT_WHITE: u64 = 142867;
const HPERFT_BLACK: u64 = 285709;

fn do_perft(b: &mut Board, depth: usize) -> u64 {
    if depth == 0 {
        return 1;
    }
    let move_gen = MoveGen::new(b);
    let mut moves = MoveList::new();
    move_gen.gen_all(&mut moves);
    moves.retain(|m| unsafe { m.is_legal_unchecked(b) });
    if depth == 1 {
        moves.len() as u64
    } else {
        moves
            .into_iter()
            .map(|mv| {
                let u = unsafe { b.make_move_unchecked(mv) };
                let res = do_perft(b, depth - 1);
                unsafe {
                    b.unmake_move_unchecked(mv, u);
                }
                res
            })
            .sum()
    }
}

fn do_hperft(b: &mut Board, depth: usize) -> u64 {
    if depth == 0 {
        let white: u64 = b.color(Color::White).flipped_rank().into();
        let black: u64 = b.color(Color::Black).flipped_rank().into();
        return white
            .wrapping_mul(crate::HPERFT_WHITE)
            .wrapping_add(black.wrapping_mul(crate::HPERFT_BLACK));
    }

    let mut result: u64 = 0;
    let move_gen = MoveGen::new(b);
    let mut moves = MoveList::new();
    move_gen.gen_all(&mut moves);
    moves.retain(|m| unsafe { m.is_legal_unchecked(b) });
    for mv in &moves {
        let u = unsafe { b.make_move_unchecked(*mv) };
        result = result.wrapping_add(do_hperft(b, depth - 1));
        unsafe {
            b.unmake_move_unchecked(*mv, u);
        }
    }
    result
}

pub struct Case {
    pub name: &'static str,
    pub fen: &'static str,
    pub depth: usize,
    pub perft: u64,
    pub hperft: u64,
}

impl Case {
    pub fn run_perft(&self) {
        let mut b = Board::from_str(self.fen).unwrap();
        assert_eq!(do_perft(&mut b, self.depth), self.perft);
    }

    pub fn run_hperft(&self) {
        let mut b = Board::from_str(self.fen).unwrap();
        assert_eq!(do_hperft(&mut b, self.depth), self.hperft);
    }
}

// Positions named jordan_* are taken from https://github.com/jordanbray/chess_perft repo.
// You can view them at
// https://github.com/jordanbray/chess_perft/blob/bbe794544cdac3b8f653fc370eea7c859b7f29aa/benches/benches.rs
pub const CASES: [Case; 36] = [
    Case {
        name: "jordan_1",
        fen: "8/5bk1/8/2Pp4/8/1K6/8/8 w - d6 0 1",
        depth: 6,
        perft: 824064,
        hperft: 10227354081862064469,
    },
    Case {
        name: "jordan_2",
        fen: "8/8/1k6/8/2pP4/8/5BK1/8 b - d3 0 1",
        depth: 6,
        perft: 824064,
        hperft: 14960676359275113292,
    },
    Case {
        name: "jordan_3",
        fen: "8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1",
        depth: 6,
        perft: 1440467,
        hperft: 1507229866844926637,
    },
    Case {
        name: "jordan_4",
        fen: "8/5k2/8/2Pp4/2B5/1K6/8/8 w - d6 0 1",
        depth: 6,
        perft: 1440467,
        hperft: 15087435520595628865,
    },
    Case {
        name: "jordan_5",
        fen: "5k2/8/8/8/8/8/8/4K2R w K - 0 1",
        depth: 6,
        perft: 661072,
        hperft: 15048005469914942504,
    },
    Case {
        name: "jordan_6",
        fen: "4k2r/8/8/8/8/8/8/5K2 b k - 0 1",
        depth: 6,
        perft: 661072,
        hperft: 15950583300412830639,
    },
    Case {
        name: "jordan_7",
        fen: "3k4/8/8/8/8/8/8/R3K3 w Q - 0 1",
        depth: 6,
        perft: 803711,
        hperft: 16122014333932527266,
    },
    Case {
        name: "jordan_8",
        fen: "r3k3/8/8/8/8/8/8/3K4 b q - 0 1",
        depth: 6,
        perft: 803711,
        hperft: 14451999952613291999,
    },
    Case {
        name: "jordan_9",
        fen: "r3k2r/1b4bq/8/8/8/8/7B/R3K2R w KQkq - 0 1",
        depth: 4,
        perft: 1274206,
        hperft: 4641921541217416058,
    },
    Case {
        name: "jordan_10",
        fen: "r3k2r/7b/8/8/8/8/1B4BQ/R3K2R b KQkq - 0 1",
        depth: 4,
        perft: 1274206,
        hperft: 14390205955143878532,
    },
    Case {
        name: "jordan_11",
        fen: "r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1",
        depth: 4,
        perft: 1720476,
        hperft: 15236009764005919001,
    },
    Case {
        name: "jordan_12",
        fen: "r3k2r/8/5Q2/8/8/3q4/8/R3K2R w KQkq - 0 1",
        depth: 4,
        perft: 1720476,
        hperft: 10737207666897534640,
    },
    Case {
        name: "jordan_13",
        fen: "2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1",
        depth: 6,
        perft: 3821001,
        hperft: 13688754110556353923,
    },
    Case {
        name: "jordan_14",
        fen: "3K4/8/8/8/8/8/4p3/2k2R2 b - - 0 1",
        depth: 6,
        perft: 3821001,
        hperft: 12511139674264896147,
    },
    Case {
        name: "jordan_15",
        fen: "8/8/1P2K3/8/2n5/1q6/8/5k2 b - - 0 1",
        depth: 5,
        perft: 1004658,
        hperft: 8828821598830464170,
    },
    Case {
        name: "jordan_16",
        fen: "5K2/8/1Q6/2N5/8/1p2k3/8/8 w - - 0 1",
        depth: 5,
        perft: 1004658,
        hperft: 410996523585496144,
    },
    Case {
        name: "jordan_17",
        fen: "4k3/1P6/8/8/8/8/K7/8 w - - 0 1",
        depth: 6,
        perft: 217342,
        hperft: 9965890832820219649,
    },
    Case {
        name: "jordan_18",
        fen: "8/k7/8/8/8/8/1p6/4K3 b - - 0 1",
        depth: 6,
        perft: 217342,
        hperft: 3000710645582169111,
    },
    Case {
        name: "jordan_19",
        fen: "8/P1k5/K7/8/8/8/8/8 w - - 0 1",
        depth: 6,
        perft: 92683,
        hperft: 6678614880857970379,
    },
    Case {
        name: "jordan_20",
        fen: "8/8/8/8/8/k7/p1K5/8 b - - 0 1",
        depth: 6,
        perft: 92683,
        hperft: 2522239085604426516,
    },
    Case {
        name: "jordan_21",
        fen: "K1k5/8/P7/8/8/8/8/8 w - - 0 1",
        depth: 6,
        perft: 2217,
        hperft: 965492357329846272,
    },
    Case {
        name: "jordan_22",
        fen: "8/8/8/8/8/p7/8/k1K5 b - - 0 1",
        depth: 6,
        perft: 2217,
        hperft: 10996353781449742,
    },
    Case {
        name: "jordan_23",
        fen: "8/k1P5/8/1K6/8/8/8/8 w - - 0 1",
        depth: 7,
        perft: 567584,
        hperft: 16246619589065769502,
    },
    Case {
        name: "jordan_24",
        fen: "8/8/8/8/1k6/8/K1p5/8 b - - 0 1",
        depth: 7,
        perft: 567584,
        hperft: 13529881500339651654,
    },
    Case {
        name: "jordan_25",
        fen: "8/8/2k5/5q2/5n2/8/5K2/8 b - - 0 1",
        depth: 4,
        perft: 23527,
        hperft: 17574558369869797364,
    },
    Case {
        name: "jordan_26",
        fen: "8/5k2/8/5N2/5Q2/2K5/8/8 w - - 0 1",
        depth: 4,
        perft: 23527,
        hperft: 3863984453770373253,
    },
    Case {
        name: "jordan_kiwipete",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 4,
        perft: 4085603,
        hperft: 13273887749508334423,
    },
    Case {
        name: "initial",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 4,
        perft: 197281,
        hperft: 3599811434478483528,
    },
    Case {
        name: "sicilian",
        fen: "r1b1k2r/2qnbppp/p2ppn2/1p4B1/3NPPP1/2N2Q2/PPP4P/2KR1B1R w kq - 0 11",
        depth: 4,
        perft: 2317898,
        hperft: 12556082293325863556,
    },
    Case {
        name: "middle",
        fen: "1rq1r1k1/1p3ppp/pB3n2/3ppP2/Pbb1P3/1PN2B2/2P2QPP/R1R4K w - - 1 21",
        depth: 4,
        perft: 2579062,
        hperft: 14747377813079023145,
    },
    Case {
        name: "open_position",
        fen: "4r1k1/3R1ppp/8/5P2/p7/6PP/4pK2/1rN1B3 w - - 4 43",
        depth: 4,
        perft: 505064,
        hperft: 4437275209935405760,
    },
    Case {
        name: "queen",
        fen: "6K1/8/8/1k3q2/3Q4/8/8/8 w - - 0 1",
        depth: 4,
        perft: 211187,
        hperft: 12245621721721354430,
    },
    Case {
        name: "pawn_move",
        fen: "4k3/pppppppp/8/8/8/8/PPPPPPPP/4K3 w - - 0 1",
        depth: 5,
        perft: 1683597,
        hperft: 7982926558036843904,
    },
    Case {
        name: "pawn_attack",
        fen: "4k3/8/8/pppppppp/PPPPPPPP/8/8/4K3 w - - 0 1",
        depth: 5,
        perft: 1370744,
        hperft: 11192399975994366848,
    },
    Case {
        name: "pawn_promote",
        fen: "8/PPPPPPPP/8/2k1K3/8/8/pppppppp/8 w - - 0 1",
        depth: 4,
        perft: 1768584,
        hperft: 8207604282890666228,
    },
    Case {
        name: "cydonia",
        fen: "5K2/1N1N1N2/8/1N1N1N2/1n1n1n2/8/1n1n1n2/5k2 w - - 0 1",
        depth: 4,
        perft: 1962254,
        hperft: 7983221043579845606,
    },
];

#[test]
fn test_perft() {
    for case in &CASES {
        case.run_perft();
    }
}

#[test]
fn test_hperft() {
    for case in &CASES {
        case.run_hperft();
    }
}
