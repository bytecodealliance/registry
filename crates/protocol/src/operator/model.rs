use crate::registry::RecordId;
use core::fmt;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
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
    fn contents(&self) -> IndexSet<&AnyHash> {
        Default::default()
    }
}

/// Each permission represents the ability to use the specified entry
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Permission {
    /// Permission to sign checkpoints.
    Commit,
    /// Permission to define namespace in operator log.
    DefineNamespace,
    /// Permission to import namespace from another registry and add to the operator log.
    ImportNamespace,
}

impl Permission {
    /// Gets an array of all permissions.
    pub const fn all() -> [Permission; 3] {
        [
            Permission::Commit,
            Permission::DefineNamespace,
            Permission::ImportNamespace,
        ]
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Permission::Commit => write!(f, "commit"),
            Permission::DefineNamespace => write!(f, "defineNamespace"),
            Permission::ImportNamespace => write!(f, "importNamespace"),
        }
    }
}

impl FromStr for Permission {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "commit" => Ok(Permission::Commit),
            "defineNamespace" => Ok(Permission::DefineNamespace),
            "importNamespace" => Ok(Permission::ImportNamespace),
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
        permissions: Vec<Permission>,
    },
    /// Remove a permission from a key.
    /// The author of this entry must have the permission.
    RevokeFlat {
        key_id: signing::KeyID,
        permissions: Vec<Permission>,
    },
    /// The registry defines a namespace to be used in its own package logs.
    DefineNamespace { namespace: String },
    /// The registry defines a namespace as imported from another registry.
    ImportNamespace { namespace: String, registry: String },
}

impl OperatorEntry {
    /// Check permission is required to submit this entry
    pub fn required_permission(&self) -> Option<Permission> {
        match self {
            Self::Init { .. } => None,
            Self::GrantFlat { .. } | Self::RevokeFlat { .. } => Some(Permission::Commit),
            Self::DefineNamespace { .. } => Some(Permission::DefineNamespace),
            Self::ImportNamespace { .. } => Some(Permission::ImportNamespace),
        }
    }
}
