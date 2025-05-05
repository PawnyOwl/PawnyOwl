use anyhow::{Context, Result, bail};
use std::{
    borrow::Borrow,
    cmp::Ordering,
    collections::HashSet,
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
};

#[derive(Debug)]
#[repr(transparent)]
pub struct Name(str);

impl Name {
    #[inline]
    fn iter_low(&self) -> impl Iterator<Item = u8> + '_ {
        self.0.bytes().map(|b| b.to_ascii_lowercase())
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Default)]
#[repr(transparent)]
pub struct NameBuf(String);

impl NameBuf {
    #[inline]
    pub fn as_name(&self) -> &Name {
        self.0.as_str().into()
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[inline]
    pub fn get(&self) -> &String {
        &self.0
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl Borrow<Name> for NameBuf {
    #[inline]
    fn borrow(&self) -> &Name {
        self.as_name()
    }
}

impl ToOwned for Name {
    type Owned = NameBuf;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        self.into()
    }
}

impl fmt::Display for Name {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for NameBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_name().fmt(f)
    }
}

impl From<NameBuf> for String {
    #[inline]
    fn from(val: NameBuf) -> Self {
        val.0
    }
}

impl From<&str> for &Name {
    #[inline]
    fn from(val: &str) -> Self {
        unsafe { &*(val as *const str as *const Name) }
    }
}

impl From<&str> for NameBuf {
    #[inline]
    fn from(val: &str) -> Self {
        Self(String::from(val))
    }
}

impl From<String> for NameBuf {
    #[inline]
    fn from(val: String) -> Self {
        Self(val)
    }
}

impl From<&Name> for NameBuf {
    #[inline]
    fn from(val: &Name) -> Self {
        val.0.into()
    }
}

impl PartialEq for Name {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.iter_low().eq(other.iter_low())
    }
}

impl PartialEq for NameBuf {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_name().eq(other.as_name())
    }
}

impl Eq for Name {}
impl Eq for NameBuf {}

impl Ord for Name {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter_low().cmp(other.iter_low())
    }
}

impl Ord for NameBuf {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_name().cmp(other.as_name())
    }
}

impl PartialOrd for Name {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd for NameBuf {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Name {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        for b in self.0.bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
    }
}

impl Hash for NameBuf {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_name().hash(state)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Val {
    Bool(bool),
    Int(i64),
    Str(String),
    Action,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Opt {
    Bool {
        val: bool,
    },
    Int {
        val: i64,
        min: Option<i64>,
        max: Option<i64>,
    },
    Enum {
        val: NameBuf,
        choice: HashSet<NameBuf>,
    },
    Str {
        val: String,
    },
    Action,
}

impl Opt {
    pub fn get(&self) -> Val {
        match self {
            Self::Bool { val } => Val::Bool(*val),
            Self::Int { val, .. } => Val::Int(*val),
            Self::Enum { val, .. } => Val::Str(val.get().clone()),
            Self::Str { val } => Val::Str(val.clone()),
            Self::Action => Val::Action,
        }
    }

    pub fn parse(&self, s: &str) -> Result<Val> {
        match self {
            Self::Bool { .. } => Ok(Val::Bool(match s {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => bail!("bad bool: {:?}", s),
            })),
            Self::Int { .. } => Ok(Val::Int(i64::from_str(s).context("parsing int option")?)),
            Self::Enum { .. } | Self::Str { .. } => {
                Ok(Val::Str(if s == "<empty>" { "".into() } else { s.into() }))
            }
            Self::Action => Ok(Val::Action),
        }
    }

    pub fn set(&mut self, v: Val) -> Result<()> {
        match self {
            Self::Bool { val } => {
                if let Val::Bool(src) = v {
                    *val = src;
                } else {
                    bail!("bool expected");
                }
            }
            Self::Int { val, min, max } => {
                if let Val::Int(src) = v {
                    if min.unwrap_or(i64::MIN) > src || max.unwrap_or(i64::MAX) < src {
                        bail!("int option out of bounds")
                    }
                    *val = src;
                } else {
                    bail!("int expected");
                }
            }
            Self::Enum { val, choice } => {
                if let Val::Str(src) = v {
                    let src: &Name = (*src).into();
                    if !choice.contains(src) {
                        bail!("bad choice value");
                    }
                    *val = src.into();
                } else {
                    bail!("str expected");
                }
            }
            Self::Str { val } => {
                if let Val::Str(src) = v {
                    *val = (*src).into();
                } else {
                    bail!("str expected");
                }
            }
            Self::Action => {
                if !matches!(v, Val::Action) {
                    bail!("action expected");
                }
            }
        }
        Ok(())
    }
}
