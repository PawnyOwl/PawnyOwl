pub mod opts;
pub mod score;

pub use score::{BoundedScore, Score};

use anyhow::Result;
use opts::Opt;
use pawnyowl_board::{Board, Move};
use std::{collections::HashMap, num::NonZeroU32, sync::Arc, time::Duration};

#[derive(Clone, Debug)]
pub struct EngineMeta {
    pub name: String,
    pub author: String,
}

#[derive(Copy, Clone, Debug)]
pub struct SearchResult {
    pub best: Move,
    pub ponder: Move,
}

#[derive(Copy, Clone, Debug)]
pub struct TimeControlSide {
    pub time: Duration,
    pub inc: Duration,
}

#[derive(Copy, Clone, Debug)]
pub struct TimeControl {
    pub white: TimeControlSide,
    pub black: TimeControlSide,
    pub moves_to_go: Option<NonZeroU32>,
}

#[derive(Copy, Clone, Debug)]
pub enum SearchConstraint {
    Infinite,
    FixedDepth(usize),
    FixedTime(Duration),
    TimeControl(TimeControl),
}

#[derive(Clone, Debug)]
pub struct SearchInfo {
    pub depth: usize,
    pub pv: Vec<Move>,
    pub score: BoundedScore,
}

pub trait Monitor: Sync {
    fn is_stopped(&self) -> bool;
    fn wait_stop(&self, time: Duration) -> bool;

    fn report_str(&self, s: &str);
    fn report_info(&self, i: &SearchInfo);
    fn report_nodes(&self, nodes: u64);
    fn report_cur_move(&self, m: Move, num: usize);
}

pub trait Engine {
    fn meta(&self) -> &EngineMeta;
    fn opts(&self) -> &HashMap<opts::NameBuf, Opt>;
    fn set_opt(&self, name: &opts::Name, val: &opts::Val<'_>);
    fn set_debug(&mut self, value: bool);
    fn on_new_game(&mut self);
    fn set_position(&mut self, b: &Board, ms: &[Move]) -> Result<()>;
    fn search(&mut self, c: SearchConstraint, mon: &Arc<dyn Monitor>) -> SearchResult;
    fn q_search(&mut self) -> Score;
}
