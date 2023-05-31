use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

mod dynamic;
mod r#static;

pub use digest::{Digest, Output};
pub use dynamic::{AnyHash, AnyHashError};
pub use r#static::Hash;
pub use sha2::Sha256;

use self::r#static::IncorrectLengthError;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

impl<D: SupportedDigest> From<Hash<D>> for AnyHash {
    fn from(value: Hash<D>) -> Self {
        AnyHash {
            algo: D::ALGORITHM,
            bytes: value.digest.to_vec(),
        }
    }
}

#[derive(Error, Debug)]
pub enum HashError {
    #[error("mismatched hash algorithm: expected {expected}, got {actual}")]
    MismatchedAlgorithms {
        expected: HashAlgorithm,
        actual: HashAlgorithm,
    },

    #[error("expected {expected} bytes for hash algorithm {algo}, got {actual}")]
    IncorrectLength {
        expected: usize,
        algo: HashAlgorithm,
        actual: usize,
    },
}

impl<D: SupportedDigest> TryFrom<AnyHash> for Hash<D> {
    type Error = HashError;

    fn try_from(value: AnyHash) -> Result<Self, Self::Error> {
        if value.algorithm() == D::ALGORITHM {
            let len = value.bytes.len();
            match Hash::try_from(value.bytes) {
                Ok(hash) => Ok(hash),
                Err(IncorrectLengthError) => Err(HashError::IncorrectLength {
                    expected: <D as Digest>::output_size(),
                    algo: D::ALGORITHM,
                    actual: len,
                }),
            }
        } else {
            Err(HashError::MismatchedAlgorithms {
                expected: D::ALGORITHM,
                actual: value.algorithm(),
            })
        }
    }
}
