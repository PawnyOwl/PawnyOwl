use anyhow::{Context, Result};
use pawnyowl::{engine::Engine, uci};
use std::io::{self, Write};

struct Warn<'a>(&'a mut dyn Write);

impl uci::Warn for Warn<'_> {
    fn warn(&mut self, msg: &str) {
        for ln in msg.lines() {
            let _ = writeln!(self.0, "warning: {}", ln);
        }
    }
}

fn main() -> Result<()> {
    uci::comm(
        &mut io::stdin().lock(),
        &mut io::stdout(),
        &mut Warn(&mut io::stderr().lock()),
        &mut Engine::new(),
    )
    .context("running engine")?;
    Ok(())
}
