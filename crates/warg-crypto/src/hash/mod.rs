use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

mod dynamic;
mod r#static;

pub use digest::{Digest, Output};
pub use sha2::Sha256;

pub use dynamic::{DynHash, DynHashParseError};
pub use r#static::Hash;

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum HashAlgorithm {
    Sha256,
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashAlgorithm::Sha256 => write!(f, "sha256"),
        }
    }
}

impl fmt::Debug for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl FromStr for HashAlgorithm {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sha256" => Ok(HashAlgorithm::Sha256),
            _ => Err(Error::msg(format!("Illegal hash algorithm '{}'", s))),
        }
    }
}

pub trait SupportedDigest: Digest + private::Sealed {
    const ALGORITHM: HashAlgorithm;
}

impl SupportedDigest for Sha256 {
    const ALGORITHM: HashAlgorithm = HashAlgorithm::Sha256;
}

mod private {
    use sha2::Sha256;

    pub trait Sealed {}
    impl Sealed for Sha256 {}
}

impl<D: SupportedDigest> From<Hash<D>> for DynHash {
    fn from(value: Hash<D>) -> Self {
        DynHash {
            algo: D::ALGORITHM,
            bytes: value.digest.to_vec(),
        }
    }
}
