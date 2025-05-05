mod comm;
mod io;
mod sanitize;
mod util;

pub trait Warn {
    fn warn(&mut self, msg: &str);
}

pub use comm::comm;
