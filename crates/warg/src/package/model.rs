use core::fmt;
use std::str::FromStr;

use crate::things::{envelope::Envelope, hash, signing::Key, Version};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Permission {
    UpdateAuth,
    Release,
    Yank,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Permission::UpdateAuth => write!(f, "update-auth"),
            Permission::Release => write!(f, "release"),
            Permission::Yank => write!(f, "yank"),
        }
    }
}

impl FromStr for Permission {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "update-auth" => Ok(Permission::UpdateAuth),
            "release" => Ok(Permission::Release),
            "yank" => Ok(Permission::Yank),
            _ => Err(()),
        }
    }
}

pub enum Entry {
    Init {
        hash_algorithm: hash::Algorithm,
        key: Key,
    },
    UpdateAuth {
        key: Key,
        allow: Vec<Permission>,
        deny: Vec<Permission>,
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
            Entry::UpdateAuth { .. } => Some(Permission::UpdateAuth),
            Entry::Release { .. } => Some(Permission::Release),
            Entry::Yank { .. } => Some(Permission::Yank),
        }
    }
}
