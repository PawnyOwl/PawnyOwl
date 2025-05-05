pub mod opts;
pub mod score;

pub use score::{BoundedScore, Score};

use opts::{Name, NameBuf, Opt, Val};
use pawnyowl_board::{Board, Move};
use std::{collections::HashMap, num::NonZeroU32, time::Duration};

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
    pub nodes: Option<u64>,
}

pub type StopCallback = Box<dyn FnOnce() + Send>;

pub trait Monitor: Sync {
    fn is_stopped(&self) -> bool;
    fn register_on_stop(&self, callback: StopCallback);

    fn report_str(&self, s: &str);
    fn report_info(&self, i: &SearchInfo);
    fn report_nodes(&self, nodes: u64);
    fn report_cur_move(&self, m: Move, num: usize);
}

pub trait Engine {
    fn meta(&self) -> EngineMeta;
    fn opts(&self) -> &HashMap<NameBuf, Opt>;
    fn set_opt(&mut self, name: &Name, val: Val);
    fn set_debug(&mut self, value: bool);
    fn on_new_game(&mut self);
    fn set_position(&mut self, b: &Board, ms: &[Move]);
    fn search(&mut self, c: SearchConstraint, mon: &dyn Monitor) -> SearchResult;
    fn q_search(&mut self) -> Score;
}
