use core::fmt;
use std::{str::FromStr, time::SystemTime};

use crate::hash;
use crate::signing;

/// A operator record is a collection of entries published together by the same author
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorRecord {
    /// The hash of the previous operator record envelope
    pub prev: Option<hash::Digest>,
    /// The version of the registry protocol used
    pub version: u32,
    /// When this record was published
    pub timestamp: SystemTime,
    /// The entries being published in this record
    pub entries: Vec<OperatorEntry>,
}

/// Each permission represents the ability to use the specified entry
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum Permission {
    Commit,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Permission::Commit => write!(f, "commit"),
        }
    }
}

impl FromStr for Permission {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "commit" => Ok(Permission::Commit),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OperatorEntry {
    /// Initializes a operator log.
    /// Must be the first entry of every log and not appear elsewhere.
    Init {
        /// The hash algorithm this log will use for linking
        hash_algorithm: hash::HashAlgorithm,
        /// The original operator key
        key: signing::PublicKey,
    },
    /// Grant the specified key a permission.
    /// The author of this entry must have the permission.
    GrantFlat {
        key: signing::PublicKey,
        permission: Permission,
    },
    /// Remove a permission from a key.
    /// The author of this entry must have the permission.
    RevokeFlat {
        key_id: signing::KeyID,
        permission: Permission,
    },
}