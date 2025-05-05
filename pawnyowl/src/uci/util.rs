use crate::intf::{
    Engine, StopCallback,
    opts::{Name, NameBuf, Val},
};
use crate::uci::io::Position;
use std::{
    collections::HashMap,
    mem,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
pub struct DelayedState {
    debug: Option<bool>,
    opts: HashMap<NameBuf, Val>,
    new_game: bool,
    position: Option<Box<Position>>,
}

impl DelayedState {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_new_game(&mut self) {
        self.new_game = true;
    }

    pub fn set_position(&mut self, position: Box<Position>) {
        self.position = Some(position);
    }

    pub fn set_debug(&mut self, val: bool) {
        self.debug = Some(val);
    }

    pub fn set_opt(&mut self, name: &Name, val: Val) {
        self.opts.insert(name.to_owned(), val);
    }

    pub fn apply(&mut self, engine: &mut (impl Engine + ?Sized)) {
        if let Some(debug) = self.debug.take() {
            engine.set_debug(debug);
        }
        for (name, val) in self.opts.drain() {
            engine.set_opt(name.as_name(), val);
        }
        if mem::replace(&mut self.new_game, false) {
            engine.on_new_game();
        }
        if let Some(position) = self.position.take() {
            engine.set_position(&position.board, &position.moves[..]);
        }
    }
}

pub struct StopState {
    is_stopped: AtomicBool,
    on_stop: Mutex<Option<Vec<StopCallback>>>,
}

impl StopState {
    pub fn new() -> Self {
        Self {
            is_stopped: AtomicBool::new(false),
            on_stop: Mutex::new(Some(Vec::new())),
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Acquire)
    }

    pub fn stop(&self) {
        if self.is_stopped.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut on_stop = self.on_stop.lock().unwrap();
        for cb in on_stop.take().unwrap() {
            cb();
        }
    }

    pub fn register_on_stop(&self, callback: StopCallback) {
        if self.is_stopped() {
            callback();
            return;
        }
        let mut on_stop = self.on_stop.lock().unwrap();
        if self.is_stopped() {
            drop(on_stop);
            callback();
            return;
        }
        on_stop.as_mut().unwrap().push(Box::new(callback));
    }
}
