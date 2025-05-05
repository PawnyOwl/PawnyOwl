use crate::intf::opts::{Name, NameBuf, Opt};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;

fn do_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("empty name");
    }
    for c in name.chars() {
        let c = c as u32;
        if !(0x20..0x7f).contains(&c) {
            bail!("char out of range");
        }
    }
    if name.starts_with(' ') {
        bail!("string has leading space");
    }
    if name.ends_with(' ') {
        bail!("string has trailing space");
    }
    if name.contains("  ") {
        bail!("string has double spaces");
    }
    Ok(())
}

fn do_choice(c: &Name) -> Result<()> {
    let c = c.as_str();
    if c == "<empty>" {
        bail!("bad value \"<empty>\"")
    }
    do_name(c)?;
    if c.contains("name") || c.contains("value") || c.contains("var") {
        bail!("string contains forbidden substrings")
    }
    Ok(())
}

pub fn opt_name(name: &Name) -> Result<()> {
    let name = name.as_str();
    do_name(name)?;
    if name.contains("name") || name.contains("value") {
        bail!("string contains forbidden substrings");
    }
    Ok(())
}

pub fn opt(opt: &Opt) -> Result<()> {
    match opt {
        Opt::Bool { .. } | Opt::Action => {
            // Nothing to sanitize.
        }
        Opt::Int { val, min, max } => {
            if !(min.unwrap_or(i64::MIN) <= *val && *val <= max.unwrap_or(i64::MAX)) {
                bail!("value {} out of bounds", val);
            }
        }
        Opt::Enum { val, choice } => {
            if !choice.contains(val) {
                bail!("value {} is not in choices", val);
            }
            for c in choice {
                do_choice(c.as_name()).with_context(|| format!("in choice {}", c))?;
            }
        }
        Opt::Str { val } => {
            if val == "<empty>" {
                bail!("bad value \"<empty>\"");
            }
            for c in val.chars() {
                if c < ' ' || (c as u32) == 0x7f {
                    bail!("bad char");
                }
            }
        }
    }
    Ok(())
}

pub fn opts(opts: &HashMap<NameBuf, Opt>) -> Result<()> {
    for (name, val) in opts {
        opt_name(name.as_name()).with_context(|| format!("in option {}", name))?;
        opt(val).with_context(|| format!("in option {}", name))?;
    }
    Ok(())
}
