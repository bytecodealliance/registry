use std::{fmt, ops::Deref, str::FromStr};

use base64;
use digest::Digest as DigestTrait;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum HashAlgorithm {
    Sha256,
}

impl HashAlgorithm {
    pub fn digest(&self, content_bytes: &[u8]) -> Digest {
        let hash_bytes: Vec<u8> = match self {
            HashAlgorithm::Sha256 => {
                let mut d = sha2::Sha256::new();
                d.update(content_bytes);
                d.finalize().deref().into()
            }
        };

        Digest {
            algo: *self,
            bytes: hash_bytes,
        }
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashAlgorithm::Sha256 => write!(f, "SHA256"),
        }
    }
}

impl FromStr for HashAlgorithm {
    type Err = HashAlgorithmParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SHA256" => Ok(HashAlgorithm::Sha256),
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
pub struct Digest {
    algo: HashAlgorithm,
    bytes: Vec<u8>,
}

impl Digest {
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algo
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algo, base64::encode(&self.bytes))
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl FromStr for Digest {
    type Err = HashParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (algo_part, bytes_part) = s
            .split_once(':')
            .ok_or_else(|| HashParseError::IncorrectStructure(s.matches(':').count() + 1))?;

        let algo = algo_part.parse::<HashAlgorithm>()?;
        let bytes = base64::decode(bytes_part)?;

        Ok(Digest { algo, bytes })
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
