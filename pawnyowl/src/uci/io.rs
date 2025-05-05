use crate::intf::{
    EngineMeta, Score, SearchConstraint, SearchInfo, SearchResult, TimeControl, TimeControlSide,
    opts::{Name, NameBuf, Opt},
    score::Bound,
};
use crate::uci::{Warn, sanitize};
use anyhow::{Context, Result, anyhow};
use pawnyowl_board::{Board, Move};
use std::{
    borrow::Cow,
    error::Error,
    io::{BufRead, Write},
    num::NonZeroU32,
    str::FromStr,
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Position {
    pub board: Board,
    pub moves: Vec<Move>,
}

#[derive(Clone, Debug)]
pub enum Command {
    Uci,
    Debug(bool),
    IsReady,
    SetOption { name: NameBuf, value: String },
    NewGame,
    Position(Box<Position>),
    Go(SearchConstraint),
    Stop,
    Quit,
}

#[derive(Clone, Debug)]
pub enum Info<'a> {
    String(&'a str),
    #[allow(clippy::enum_variant_names)]
    Info {
        time: Duration,
        info: &'a SearchInfo,
    },
    Nodes {
        time: Duration,
        nodes: u64,
    },
    CurMove {
        mv: Move,
        num: usize,
    },
}

#[derive(Clone, Debug)]
pub enum Message<'a> {
    UciOk,
    Id(&'a EngineMeta),
    Option { name: &'a Name, value: &'a Opt },
    ReadyOk,
    Info(Info<'a>),
    BestMove(SearchResult),
}

fn sanitize_str(s: &str) -> Cow<'_, str> {
    const UNSAFE_CHARS: &[char] = &['\n', '\r', '\t'];
    if s.contains(UNSAFE_CHARS) {
        s.replace(UNSAFE_CHARS, " ").into()
    } else {
        s.into()
    }
}

fn calc_nps(nodes: u64, time: &Duration) -> Option<u64> {
    let us = time.as_micros();
    if us < 10_000 {
        // Too little time (< 10ms) have passed. Do not compute NPS in this case.
        return None;
    }
    let npus = (nodes as u128) / us;
    let nps = (npus + 500_000) / 1_000_000;
    nps.try_into().ok()
}

pub fn write_msg(msg: &Message, w: &mut (impl Write + ?Sized)) -> Result<()> {
    match msg {
        Message::UciOk => writeln!(w, "uciok")?,
        Message::Id(meta) => {
            writeln!(w, "id name {}", sanitize_str(&meta.name))?;
            writeln!(w, "id author {}", sanitize_str(&meta.author))?;
        }
        Message::Option { name, value } => {
            sanitize::opt_name(name).context("sanitizing option name")?;
            sanitize::opt(value).context("sanitizing option value")?;
            let mut s = format!("option name {}", name);
            match value {
                Opt::Bool { val } => s += &format!(" type check default {}", val),
                Opt::Int { val, min, max } => {
                    s += &format!(" type spin default {}", val);
                    if let Some(min) = min {
                        s += &format!(" min {}", min);
                    }
                    if let Some(max) = max {
                        s += &format!(" max {}", max);
                    }
                }
                Opt::Enum { val, choice } => {
                    s += &format!(" type combo default {}", val);
                    for c in choice {
                        s += &format!(" var {}", c);
                    }
                }
                Opt::Str { val } => {
                    let val = if val.is_empty() {
                        "<empty>".into()
                    } else {
                        sanitize_str(val)
                    };
                    s += &format!(" type string default {}", &val);
                }
                Opt::Action => s += " type button",
            }
            writeln!(w, "{}", &s)?;
        }
        Message::ReadyOk => writeln!(w, "readyok")?,
        Message::Info(info) => match info {
            Info::String(s) => writeln!(w, "info string {}", sanitize_str(s))?,
            Info::Info { time, info } => {
                let mut s = format!("info depth {} time {}", info.depth, time.as_millis());
                if let Some(nodes) = info.nodes {
                    s += &format!(" nodes {}", nodes);
                    if let Some(nps) = calc_nps(nodes, time) {
                        s += &format!(" nps {}", nps);
                    }
                }
                if !info.pv.is_empty() {
                    let pv = info.pv.iter().map(ToString::to_string).collect::<Vec<_>>();
                    s += &format!(" pv {}", pv.join(" "));
                }
                match info.score.score {
                    Score::Cp(cp) => s += &format!(" score cp {}", cp),
                    Score::Mate { moves, win } => {
                        let mate = (moves as i64) * (if win { 1 } else { -1 });
                        s += &format!(" score mate {}", mate);
                    }
                }
                match info.score.bound {
                    Bound::Exact => {}
                    Bound::Lower => s += " lowerbound",
                    Bound::Upper => s += " upperbound",
                }
                writeln!(w, "{}", &s)?;
            }
            Info::Nodes { time, nodes } => {
                let mut s = format!("info time {} nodes {}", time.as_millis(), nodes);
                if let Some(nps) = calc_nps(*nodes, time) {
                    s += &format!(" nps {}", nps);
                }
                writeln!(w, "{}", &s)?;
            }
            Info::CurMove { mv, num } => {
                writeln!(w, "info currmove {} currmovenumber {}", mv, num)?
            }
        },
        Message::BestMove(res) => {
            if res.ponder == Move::NULL {
                writeln!(w, "bestmove {}", res.best)?;
            } else {
                writeln!(w, "bestmove {} ponder {}", res.best, res.ponder)?;
            }
        }
    }
    Ok(())
}

fn parse_position<'a>(
    mut tokens: impl Iterator<Item = &'a str>,
    warn: &mut dyn Warn,
) -> Option<Box<Position>> {
    let board = match tokens.next() {
        Some("startpos") => {
            loop {
                match tokens.next() {
                    Some("moves") => break,
                    None => {
                        warn.warn("\"moves\" expected");
                        break;
                    }
                    Some(token) => warn.warn(&format!("\"moves\" expected, {:?} found", token)),
                }
            }
            Board::start()
        }
        Some("fen") => {
            let mut fen_tokens = Vec::new();
            loop {
                match tokens.next() {
                    Some("moves") => break,
                    None => {
                        warn.warn("\"moves\" expected");
                        break;
                    }
                    Some(token) => fen_tokens.push(token),
                }
            }
            match Board::from_str(&fen_tokens.join(" ")) {
                Ok(b) => b,
                Err(e) => {
                    warn.warn(&format!("bad fen: {}", e));
                    return None;
                }
            }
        }
        Some(_) => {
            warn.warn("\"startpos\" or \"fen\" expected");
            return None;
        }
        None => {
            warn.warn("no position");
            return None;
        }
    };

    let mut tmp_board = board.clone();
    let mut moves = Vec::new();
    for (i, token) in tokens.enumerate() {
        let mv = match Move::from_uci_legal(token, &tmp_board) {
            Ok(mv) => mv,
            Err(e) => {
                warn.warn(&format!("bad move #{} {:?}: {}", i + 1, token, e));
                return None;
            }
        };
        moves.push(mv);
        unsafe { tmp_board.make_move_unchecked(mv) };
    }

    Some(Box::new(Position { board, moves }))
}

fn parse_int<T: FromStr>(token: Option<&str>) -> Result<T>
where
    <T as FromStr>::Err: Error + Send + Sync + 'static,
{
    match token {
        Some(token) => Ok(T::from_str(token)?),
        None => Err(anyhow!("no value")),
    }
}

fn parse_msec(token: Option<&str>) -> Result<Duration> {
    Ok(Duration::from_millis(parse_int(token)?))
}

fn parse_go<'a>(
    mut tokens: impl Iterator<Item = &'a str>,
    warn: &mut dyn Warn,
) -> Option<SearchConstraint> {
    const SUBCOMMANDS: &[&str] = &[
        "searchmoves",
        "ponder",
        "wtime",
        "btime",
        "winc",
        "binc",
        "movestogo",
        "depth",
        "nodes",
        "mate",
        "movetime",
        "infinite",
    ];

    // We don't try to support some weird combination of parameters here. Instead, we follow the
    // simple logic described below.
    //
    // First, try to search for "depth", "movetime" or "infinite" options and use first of them
    // found. Otherwise, assume that we use a time control and look up for the corresponding
    // options. If they are also not found, assume infinite search.
    //
    // Such behavior might cause bugs in GUIs in some weird cases. If that happens, feel free to
    // adjust the logic or submit an issue.
    let mut time_control = None;
    let mut constraint = None;
    let default_time_control = || {
        let side = TimeControlSide {
            time: Duration::from_secs(30 * 60), // Assume 30 minutes if not specified.
            inc: Duration::ZERO,
        };
        TimeControl {
            white: side,
            black: side,
            moves_to_go: None,
        }
    };
    macro_rules! force_time_control {
        () => {
            time_control.get_or_insert_with(default_time_control)
        };
    }
    loop {
        let mut token = tokens.next();
        if token == Some("searchmoves") {
            loop {
                token = tokens.next();
                match token {
                    Some(token) => {
                        if SUBCOMMANDS.contains(&token) {
                            break;
                        }
                    }
                    None => break,
                }
            }
            // We have read an extra token here. Fall through to parse it.
        }
        match token {
            Some("searchmoves") => warn.warn("two \"searchmoves\" in a row"),
            Some("ponder") => {
                // Not supported.
            }
            Some("wtime") => match parse_msec(tokens.next()) {
                Ok(t) => force_time_control!().white.time = t,
                Err(e) => warn.warn(&format!("bad \"wtime\": {}", e)),
            },
            Some("btime") => match parse_msec(tokens.next()) {
                Ok(t) => force_time_control!().black.time = t,
                Err(e) => warn.warn(&format!("bad \"btime\": {}", e)),
            },
            Some("winc") => match parse_msec(tokens.next()) {
                Ok(t) => force_time_control!().white.inc = t,
                Err(e) => warn.warn(&format!("bad \"winc\": {}", e)),
            },
            Some("binc") => match parse_msec(tokens.next()) {
                Ok(t) => force_time_control!().black.inc = t,
                Err(e) => warn.warn(&format!("bad \"binc\": {}", e)),
            },
            Some("movestogo") => match parse_int(tokens.next()) {
                Ok(v) => force_time_control!().moves_to_go = NonZeroU32::new(v),
                Err(e) => warn.warn(&format!("bad \"movestogo\": {}", e)),
            },
            Some("depth") => match parse_int(tokens.next()) {
                Ok(v) => match &constraint {
                    None => constraint = Some(SearchConstraint::FixedDepth(v)),
                    Some(_) => warn.warn("\"depth\" ignored"),
                },
                Err(e) => warn.warn(&format!("bad \"depth\": {}", e)),
            },
            Some("nodes") => {
                // Not supported.
                _ = tokens.next();
            }
            Some("mate") => {
                // Not supported.
                _ = tokens.next();
            }
            Some("movetime") => match parse_msec(tokens.next()) {
                Ok(t) => match &constraint {
                    None => constraint = Some(SearchConstraint::FixedTime(t)),
                    Some(_) => warn.warn("\"movetime\" ignored"),
                },
                Err(e) => warn.warn(&format!("bad \"movetime\": {}", e)),
            },
            Some("infinite") => match &constraint {
                None => constraint = Some(SearchConstraint::Infinite),
                Some(_) => warn.warn("\"infinite\" ignored"),
            },
            Some(tok) => warn.warn(&format!("bad token: {:?}", tok)),
            None => break,
        }
    }

    if let Some(constraint) = constraint {
        Some(constraint)
    } else if let Some(time_control) = time_control {
        Some(SearchConstraint::TimeControl(time_control))
    } else {
        warn.warn("no options for \"go\", starting infinite search");
        Some(SearchConstraint::Infinite)
    }
}

pub fn read_cmd(r: &mut (impl BufRead + ?Sized), warn: &mut dyn Warn) -> Result<Option<Command>> {
    let mut ln = String::new();
    loop {
        ln.clear();
        let bytes = r.read_line(&mut ln)?;
        if bytes == 0 {
            return Ok(None);
        }
        let mut tokens = ln.split_whitespace().fuse();
        while let Some(token) = tokens.next() {
            match token {
                "uci" => {
                    if tokens.next().is_some() {
                        warn.warn("extra data in \"uci\"");
                    }
                    return Ok(Some(Command::Uci));
                }
                "debug" => {
                    let token = tokens.next();
                    let value = match token {
                        Some("on") => true,
                        Some("off") => false,
                        Some(token) => {
                            warn.warn(&format!("bad debug value: {}", token));
                            break;
                        }
                        None => {
                            warn.warn("no debug value");
                            break;
                        }
                    };
                    if tokens.next().is_some() {
                        warn.warn("extra data in \"debug\"");
                    }
                    return Ok(Some(Command::Debug(value)));
                }
                "isready" => {
                    if tokens.next().is_some() {
                        warn.warn("extra data in \"isready\"");
                    }
                    return Ok(Some(Command::IsReady));
                }
                "setoption" => {
                    if tokens.next() != Some("name") {
                        warn.warn("\"name\" expected");
                        break;
                    }
                    let mut name_tokens = Vec::new();
                    loop {
                        match tokens.next() {
                            Some("value") | None => break,
                            Some(token) => name_tokens.push(token),
                        }
                    }
                    let name = name_tokens.join(" ").into();
                    let value = tokens.collect::<Vec<_>>().join(" ");
                    return Ok(Some(Command::SetOption { name, value }));
                }
                "ucinewgame" => {
                    if tokens.next().is_some() {
                        warn.warn("extra data in \"ucinewgame\"");
                    }
                    return Ok(Some(Command::NewGame));
                }
                "position" => match parse_position(tokens, warn) {
                    Some(p) => return Ok(Some(Command::Position(p))),
                    None => break,
                },
                "go" => match parse_go(tokens, warn) {
                    Some(c) => return Ok(Some(Command::Go(c))),
                    None => break,
                },
                "stop" => {
                    if tokens.next().is_some() {
                        warn.warn("extra data in \"stop\"");
                    }
                    return Ok(Some(Command::Stop));
                }
                "quit" => {
                    if tokens.next().is_some() {
                        warn.warn("extra data in \"quit\"");
                    }
                    return Ok(Some(Command::Quit));
                }
                _ => {
                    warn.warn(&format!("bad token: {:?}", token));
                }
            }
        }
    }
}
