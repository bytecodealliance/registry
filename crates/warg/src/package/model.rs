use core::fmt;
use std::{str::FromStr, time::SystemTime};

use crate::hash;
use crate::signing;
use crate::version::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub prev: Option<hash::Hash>,
    pub version: u32,
    pub timestamp: SystemTime,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Permission {
    Release,
    Yank,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Permission::Release => write!(f, "release"),
            Permission::Yank => write!(f, "yank"),
        }
    }
}

impl FromStr for Permission {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "release" => Ok(Permission::Release),
            "yank" => Ok(Permission::Yank),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Init {
        hash_algorithm: hash::Algorithm,
        key: signing::PublicKey,
    },
    GrantFlat {
        key: signing::PublicKey,
        permission: Permission,
    },
    RevokeFlat {
        key_id: hash::Hash,
        permission: Permission,
    },
    Release {
        version: Version,
        content: hash::Hash,
    },
    Yank {
        version: Version,
    },
}

impl Entry {
    pub fn required_permission(&self) -> Option<Permission> {
        match self {
            Entry::Init { .. } => None,
            Entry::GrantFlat { .. } => None,
            Entry::RevokeFlat { .. } => None,
            Entry::Release { .. } => Some(Permission::Release),
            Entry::Yank { .. } => Some(Permission::Yank),
        }
    }
}
