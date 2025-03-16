use arrayvec::ArrayVec;
use pawnyowl_board::{Board, Color, File, Move, MoveGen, MoveList, Rank, Sq, movegen, selftest};
use std::io::{BufRead, Write};
use std::str::FromStr;

#[derive(Copy, Clone, Debug)]
pub struct Options {
    pub big_depth: bool,
    pub dump_trace_chains: bool,
    pub run_self_test: bool,
    pub attack_heatmaps: bool,
}

impl Default for Options {
    #[inline]
    fn default() -> Self {
        Self {
            big_depth: true,
            dump_trace_chains: false,
            run_self_test: true,
            attack_heatmaps: true,
        }
    }
}

pub struct Tester<'a, W> {
    options: Options,
    writer: &'a mut W,
}

struct DepthCtx<'a> {
    spec: &'a DepthSpec,
    hash: u64,
    chain: String,
}

impl<'a> DepthCtx<'a> {
    fn new(spec: &'a DepthSpec) -> Self {
        Self {
            spec,
            hash: 0,
            chain: String::new(),
        }
    }

    fn grow_hash(&mut self, val: u64) {
        self.hash = self.hash.wrapping_mul(2579);
        self.hash = self.hash.wrapping_add(val);
    }
}

struct DepthSpec {
    depth: usize,
    with_heatmaps: bool,
}

impl DepthSpec {
    fn name(&self) -> String {
        match self.with_heatmaps {
            true => format!("{}-heatmaps", self.depth),
            false => format!("{}", self.depth),
        }
    }
}

impl<'a, W: Write> Tester<'a, W> {
    pub fn new(options: Options, writer: &'a mut W) -> Self {
        Self { options, writer }
    }

    fn move_strings(&self, board: &mut Board, moves: &MoveList) -> Vec<String> {
        let mut result = Vec::with_capacity(moves.len());
        for mv in moves {
            if let Some(u) = unsafe { board.try_make_move_unchecked(*mv) } {
                result.push(mv.to_string());
                unsafe { board.unmake_move_unchecked(*mv, &u) };
            }
        }
        result.sort();
        result
    }

    fn move_hash(&self, mv: Move) -> u64 {
        let s = mv.to_string();
        assert!(matches!(s.len(), 4 | 5));
        let s = s.as_bytes();
        let mut res = (s[0] as u64 - 'a' as u64) * 512
            + (s[1] as u64 - '1' as u64) * 64
            + (s[2] as u64 - 'a' as u64) * 8
            + (s[3] as u64 - '1' as u64);
        res *= 5;
        if s.len() == 5 {
            res += match s[4] {
                b'n' => 1,
                b'b' => 2,
                b'r' => 3,
                b'q' => 4,
                c => panic!("unexpected move char {}", c as char),
            };
        }
        res
    }

    fn depth_dump(&mut self, depth: usize, board: &mut Board, ctx: &mut DepthCtx) {
        if depth == 0 {
            if self.options.dump_trace_chains {
                writeln!(self.writer, "cur-chain: {}", ctx.chain).unwrap();
            }

            if !self.options.attack_heatmaps {
                assert!(!ctx.spec.with_heatmaps);
            }
            if ctx.spec.with_heatmaps {
                for color in [Color::White, Color::Black] {
                    for y in Rank::iter() {
                        let mut data: u64 = 0;
                        for x in File::iter() {
                            data = data.wrapping_mul(2);
                            data = data.wrapping_add(movegen::is_square_attacked(
                                board,
                                Sq::make(x, y),
                                color,
                            ) as u64);
                        }
                        ctx.grow_hash(data);
                    }
                }
            }

            ctx.grow_hash(board.is_check() as u64);

            return;
        }

        let move_gen = MoveGen::new(board);
        let mut moves = MoveList::new();
        move_gen.gen_all(&mut moves);
        let mut move_ord: ArrayVec<(u64, usize), 256> = ArrayVec::new();
        for (i, mv) in moves.iter().enumerate() {
            move_ord.push((self.move_hash(*mv), i));
        }
        move_ord.sort();

        ctx.grow_hash(519365819);
        for (val, idx) in move_ord {
            let mv = moves[idx];
            let old_len = ctx.chain.len();
            if let Some(u) = unsafe { board.try_make_move_unchecked(mv) } {
                if self.options.dump_trace_chains {
                    ctx.chain += &(mv.to_string() + " ");
                }
                ctx.grow_hash(val);
                self.depth_dump(depth - 1, board, ctx);
                unsafe { board.unmake_move_unchecked(mv, &u) };
            }
            ctx.chain.truncate(old_len);
        }
        ctx.grow_hash(15967534195);
    }

    pub fn run_many<R: BufRead>(&mut self, reader: &mut R) {
        for line in reader.lines() {
            let line = line.expect("i/o error");
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            self.run_one(line);
        }
    }

    pub fn run_one(&mut self, fen: &str) {
        let mut board = Board::from_str(fen).unwrap();
        writeln!(self.writer, "fen: {}", fen).unwrap();
        if self.options.run_self_test {
            selftest::selftest(&board);
        }

        let move_gen = MoveGen::new(&board);
        let mut moves = MoveList::new();
        move_gen.gen_all(&mut moves);
        let move_strs = self.move_strings(&mut board, &moves);

        writeln!(self.writer, "moves: [").unwrap();
        for s in &move_strs {
            writeln!(self.writer, "  {}", s).unwrap();
        }
        writeln!(self.writer, "]").unwrap();

        let is_check = match board.is_check() {
            true => "true",
            false => "false",
        };
        writeln!(self.writer, "check?: {}", is_check).unwrap();

        if self.options.attack_heatmaps {
            for color in [Color::White, Color::Black] {
                let color_str = match color {
                    Color::White => "white",
                    Color::Black => "black",
                };
                writeln!(self.writer, "{}-heatmap: [", color_str).unwrap();
                for y in Rank::iter() {
                    write!(self.writer, "  ").unwrap();
                    for x in File::iter() {
                        match movegen::is_square_attacked(&board, Sq::make(x, y), color) {
                            true => write!(self.writer, "#").unwrap(),
                            false => write!(self.writer, ".").unwrap(),
                        };
                    }
                    writeln!(self.writer).unwrap();
                }
                writeln!(self.writer, "]").unwrap();
            }
        }

        if self.options.run_self_test {
            for mv in &moves {
                if let Some(u) = unsafe { board.try_make_move_unchecked(*mv) } {
                    selftest::selftest(&board);
                    unsafe { board.unmake_move_unchecked(*mv, &u) };
                }
            }
        }

        let mut specs = vec![
            DepthSpec {
                depth: 1,
                with_heatmaps: self.options.attack_heatmaps,
            },
            DepthSpec {
                depth: 2,
                with_heatmaps: false,
            },
        ];
        if self.options.big_depth {
            if self.options.attack_heatmaps {
                specs.push(DepthSpec {
                    depth: 2,
                    with_heatmaps: true,
                });
            }
            specs.push(DepthSpec {
                depth: 3,
                with_heatmaps: false,
            });
        }

        for spec in &specs {
            let mut ctx = DepthCtx::new(spec);
            self.depth_dump(spec.depth, &mut board, &mut ctx);
            writeln!(self.writer, "depth-dump-at-{}: {}", spec.name(), ctx.hash).unwrap();
        }

        writeln!(self.writer).unwrap();
    }
}
