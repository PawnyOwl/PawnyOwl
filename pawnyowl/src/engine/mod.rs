use crate::intf::{
    self, EngineMeta, Monitor, SearchConstraint, SearchResult,
    opts::{Name, NameBuf, Opt, Val},
    score::{Bound, BoundedScore, Score},
};
use pawnyowl_board::{Board, File, Move, MoveKind, Rank, Sq};
use std::{
    collections::HashMap,
    sync::mpsc::{self, RecvTimeoutError},
    time::Duration,
};

pub struct Engine {
    opts: HashMap<NameBuf, Opt>,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            opts: HashMap::new(),
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl intf::Engine for Engine {
    fn meta(&self) -> EngineMeta {
        EngineMeta {
            name: format!("PawnyOwl pre-alpha (v. {})", env!("CARGO_PKG_VERSION")),
            author: "PawnyOwl developers".into(),
        }
    }

    fn opts(&self) -> &HashMap<NameBuf, Opt> {
        &self.opts
    }

    fn set_opt(&mut self, name: &Name, val: Val) {
        self.opts.get_mut(name).unwrap().set(val).unwrap();
    }

    fn set_debug(&mut self, _value: bool) {}

    fn on_new_game(&mut self) {}

    fn set_position(&mut self, b: &Board, ms: &[Move]) {
        (_, _) = (b, ms);
    }

    fn search(&mut self, c: SearchConstraint, mon: &dyn Monitor) -> SearchResult {
        _ = c;
        let mv = Move::new(
            MoveKind::PawnDouble,
            Sq::make(File::E, Rank::R2),
            Sq::make(File::E, Rank::R4),
        )
        .unwrap();
        let (stop_send, stop) = mpsc::channel();
        mon.register_on_stop(Box::new(move || {
            let _ = stop_send.send(());
        }));
        for i in 1..=5 {
            match stop.recv_timeout(Duration::from_secs(2)) {
                Ok(_) => break,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => panic!("must not happen"),
            }
            mon.report_info(&intf::SearchInfo {
                depth: i,
                pv: vec![mv],
                score: BoundedScore {
                    score: Score::Cp(42),
                    bound: Bound::Exact,
                },
                nodes: None,
            });
        }
        SearchResult {
            best: mv,
            ponder: Move::NULL,
        }
    }

    fn q_search(&mut self) -> Score {
        Score::Cp(42)
    }
}
