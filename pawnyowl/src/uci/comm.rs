use crate::intf::{Engine, Monitor, SearchConstraint, SearchInfo, StopCallback, opts::Val};
use crate::uci::{
    Warn,
    io::{self, Command, Info, Message},
    sanitize,
    util::{DelayedState, StopState},
};
use anyhow::{Context, Result};
use pawnyowl_board::Move;
use std::{
    io::{BufRead, Write},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, ScopedJoinHandle},
    time::{Duration, Instant},
};

struct SearchMonitor<'a, 'b, 'c> {
    start: Instant,
    output: &'a Mutex<&'b mut (dyn Write + Send + Sync)>,
    stop_state: &'c StopState,
}

impl<'a, 'b, 'c> SearchMonitor<'a, 'b, 'c> {
    fn new(
        output: &'a Mutex<&'b mut (dyn Write + Send + Sync)>,
        stop_state: &'c StopState,
    ) -> Self {
        Self {
            start: Instant::now(),
            output,
            stop_state,
        }
    }

    fn time_passed(&self) -> Duration {
        Instant::now().duration_since(self.start)
    }
}

impl Monitor for SearchMonitor<'_, '_, '_> {
    fn is_stopped(&self) -> bool {
        self.stop_state.is_stopped()
    }

    fn register_on_stop(&self, callback: StopCallback) {
        self.stop_state.register_on_stop(callback);
    }

    fn report_str(&self, s: &str) {
        let mut output = self.output.lock().unwrap();
        let _ = io::write_msg(&Message::Info(Info::String(s)), *output);
    }

    fn report_info(&self, info: &SearchInfo) {
        let mut output = self.output.lock().unwrap();
        let _ = io::write_msg(
            &Message::Info(Info::Info {
                time: self.time_passed(),
                info,
            }),
            *output,
        );
    }

    fn report_nodes(&self, nodes: u64) {
        let mut output = self.output.lock().unwrap();
        let _ = io::write_msg(
            &Message::Info(Info::Nodes {
                time: self.time_passed(),
                nodes,
            }),
            *output,
        );
    }

    fn report_cur_move(&self, mv: Move, num: usize) {
        let mut output = self.output.lock().unwrap();
        let _ = io::write_msg(&Message::Info(Info::CurMove { mv, num }), *output);
    }
}

pub fn comm(
    input: &mut dyn BufRead,
    output: &mut (dyn Write + Send + Sync),
    warn: &mut dyn Warn,
    engine: &mut (dyn Engine + Send + Sync),
) -> Result<()> {
    let meta = engine.meta();
    let mut opts = engine.opts().clone();
    sanitize::opts(&opts)?;

    let output = Mutex::new(output);
    let engine = Mutex::new(engine);
    let delayed_state = Mutex::new(DelayedState::new());
    let searching = AtomicBool::new(false);
    let (go_chan, go_chan_recv) = mpsc::sync_channel::<SearchConstraint>(0);
    let (ack_chan_send, ack_chan) = mpsc::sync_channel::<Weak<StopState>>(0);

    let try_apply_delayed_state = |delayed_state: &mut DelayedState| {
        if !searching.load(Ordering::SeqCst) {
            delayed_state.apply(*engine.try_lock().unwrap());
        }
    };

    thread::scope(|scope| {
        struct GuardData {
            stop: Weak<StopState>,
        }

        // Move `go_chan` here to drop it before the thread joins.
        let go_chan = go_chan;
        let mut guard = scopeguard::guard(
            GuardData {
                stop: Default::default(),
            },
            |data| {
                if let Some(stop) = data.stop.upgrade() {
                    stop.stop();
                }
            },
        );

        let thread = scope.spawn(|| -> Result<()> {
            let go_chan = go_chan_recv;
            let ack_chan = ack_chan_send;
            while let Ok(constr) = go_chan.recv() {
                searching.store(true, Ordering::SeqCst);
                let mut engine = engine.lock().unwrap();

                let stop_state = Arc::new(StopState::new());
                ack_chan.send(Arc::downgrade(&stop_state)).unwrap();
                let res = engine.search(constr, &SearchMonitor::new(&output, &stop_state));
                drop(stop_state);

                {
                    let mut output = output.lock().unwrap();
                    io::write_msg(&Message::BestMove(res), *output)?;
                }

                let mut st = delayed_state.lock().unwrap();
                st.apply(*engine);
                // The order of drops is very important here!
                drop(engine);
                searching.store(false, Ordering::SeqCst);
                drop(st);
            }
            Ok(())
        });

        let handle_thread_death = |thread: ScopedJoinHandle<'_, Result<()>>| -> Result<()> {
            Err(thread.join().unwrap().unwrap_err()).context("running search thread")
        };

        while let Some(cmd) = io::read_cmd(input, warn).context("reading command")? {
            if thread.is_finished() {
                return handle_thread_death(thread);
            }
            match cmd {
                Command::Uci => {
                    let mut output = output.lock().unwrap();
                    io::write_msg(&Message::Id(&meta), *output)?;
                    for (name, value) in &opts {
                        io::write_msg(
                            &Message::Option {
                                name: name.as_name(),
                                value,
                            },
                            *output,
                        )?;
                    }
                    io::write_msg(&Message::UciOk, *output)?;
                }
                Command::Debug(val) => {
                    let mut st = delayed_state.lock().unwrap();
                    st.set_debug(val);
                    try_apply_delayed_state(&mut st);
                }
                Command::IsReady => {
                    let mut output = output.lock().unwrap();
                    io::write_msg(&Message::ReadyOk, *output)?;
                }
                Command::SetOption { name, value } => match opts.get_mut(&name) {
                    Some(opt) => match || -> Result<Val> {
                        let val = opt.parse(&value)?;
                        opt.set(val.clone())?;
                        Ok(val)
                    }() {
                        Ok(val) => {
                            let mut st = delayed_state.lock().unwrap();
                            st.set_opt(name.as_name(), val);
                            try_apply_delayed_state(&mut st);
                        }
                        Err(err) => warn.warn(&format!(
                            "bad value \"{}\" for option \"{}\": {}",
                            &value,
                            name.as_str(),
                            err
                        )),
                    },
                    None => warn.warn(&format!("unknown option \"{}\"", name.as_str())),
                },
                Command::NewGame => {
                    let mut st = delayed_state.lock().unwrap();
                    st.set_new_game();
                    try_apply_delayed_state(&mut st);
                }
                Command::Position(pos) => {
                    let mut st = delayed_state.lock().unwrap();
                    st.set_position(pos);
                    try_apply_delayed_state(&mut st);
                }
                Command::Go(constr) => {
                    if searching.load(Ordering::SeqCst) {
                        warn.warn("search is already running");
                    } else if let Ok(()) = go_chan.send(constr) {
                        let stop = ack_chan.recv().unwrap();
                        guard.stop = stop;
                    } else {
                        // Could not send a command to the search thread. It means that the thread
                        // has terminated with either a panic or an error.
                        return handle_thread_death(thread);
                    }
                }
                Command::Stop => {
                    if searching.load(Ordering::SeqCst) {
                        if let Some(stop) = guard.stop.upgrade() {
                            stop.stop();
                        }
                    }
                }
                Command::Quit => break,
            }
        }
        if thread.is_finished() {
            return handle_thread_death(thread);
        }
        Ok(())
    })
}
