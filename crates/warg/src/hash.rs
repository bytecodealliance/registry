use std::{fmt, ops::Deref, str::FromStr};

use base64::{decode, encode};
use digest::Digest;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum HashAlgorithm {
    SHA256,
}

impl HashAlgorithm {
    pub fn digest(&self, content_bytes: &[u8]) -> Hash {
        let hash_bytes: Vec<u8> = match self {
            HashAlgorithm::SHA256 => {
                let mut d = sha2::Sha256::new();
                d.update(content_bytes);
                d.finalize().deref().into()
            }
        };

        Hash {
            algo: *self,
            bytes: hash_bytes,
        }
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashAlgorithm::SHA256 => write!(f, "SHA256"),
        }
    }
}

impl FromStr for HashAlgorithm {
    type Err = HashAlgorithmParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SHA256" => Ok(HashAlgorithm::SHA256),
            _ => Err(HashAlgorithmParseError {
                value: s.to_owned(),
            }),
        }
    }
}

#[derive(Error, Debug)]
#[error("\"{value}\" is not a valid algorithm choice")]
pub struct HashAlgorithmParseError {
    value: String,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Hash {
    algo: HashAlgorithm,
    bytes: Vec<u8>,
}

impl Hash {
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algo.clone()
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algo, encode(&self.bytes))
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl FromStr for Hash {
    type Err = HashParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(|c| c == ':').collect();
        if parts.len() != 2 {
            return Err(HashParseError::IncorrectStructure(parts.len()));
        }
        let algo = parts[0].parse::<HashAlgorithm>()?;
        let bytes = decode(parts[1])?;

        Ok(Hash { algo, bytes })
    }
}

#[derive(Error, Debug)]
pub enum HashParseError {
    #[error("expected 2 parts, found {0}")]
    IncorrectStructure(usize),

    #[error("unable to parse hash algorithm")]
    HashAlgorithmParseError(#[from] HashAlgorithmParseError),

    #[error("base64 decode failed")]
    Base64DecodeError(#[from] base64::DecodeError),
}
