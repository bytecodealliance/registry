use crate::registry::RecordId;
use core::fmt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{str::FromStr, time::SystemTime};
use warg_crypto::hash::{AnyHash, HashAlgorithm};
use warg_crypto::signing;

/// An operator record is a collection of entries published together by the same author
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorRecord {
    /// The hash of the previous operator record envelope
    pub prev: Option<RecordId>,
    /// The version of the registry protocol used
    pub version: u32,
    /// When this record was published
    pub timestamp: SystemTime,
    /// The entries being published in this record
    pub entries: Vec<OperatorEntry>,
}

impl crate::Record for OperatorRecord {
    fn contents(&self) -> HashSet<&AnyHash> {
        Default::default()
    }
}

/// Each permission represents the ability to use the specified entry
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum OperatorPermission {
    Commit,
}

impl OperatorPermission {
    /// Gets an array of all permissions.
    pub const fn all() -> [OperatorPermission; 1] {
        [OperatorPermission::Commit]
    }
}

impl fmt::Display for OperatorPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperatorPermission::Commit => write!(f, "commit"),
        }
    }
}

impl FromStr for OperatorPermission {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "commit" => Ok(OperatorPermission::Commit),
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
        hash_algorithm: HashAlgorithm,
        /// The original operator key
        key: signing::PublicKey,
    },
    /// Grant the specified key a permission.
    /// The author of this entry must have the permission.
    GrantFlat {
        key: signing::PublicKey,
        permission: OperatorPermission,
    },
    /// Remove a permission from a key.
    /// The author of this entry must have the permission.
    RevokeFlat {
        key_id: signing::KeyID,
        permission: OperatorPermission,
    },
}

impl OperatorEntry {
    /// Check permission is required to submit this entry
    pub fn required_permission(&self) -> Option<OperatorPermission> {
        match self {
            Self::Init { .. } => None,
            Self::GrantFlat { .. } | Self::RevokeFlat { .. } => Some(OperatorPermission::Commit),
        }
    }
}
