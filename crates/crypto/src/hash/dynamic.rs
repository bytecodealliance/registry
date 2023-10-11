use super::{Digest, HashAlgorithm, Sha256};
use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Deref, str::FromStr};
use thiserror::Error;

pub enum Hasher {
    Sha256(Sha256),
}

impl Hasher {
    pub fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha256(d) => d.update(bytes),
        }
    }

    pub fn finalize(self) -> AnyHash {
        let (algo, bytes) = match self {
            Self::Sha256(d) => (HashAlgorithm::Sha256, d.finalize().deref().into()),
        };

        AnyHash { algo, bytes }
    }
}

impl HashAlgorithm {
    pub fn hasher(&self) -> Hasher {
        match self {
            HashAlgorithm::Sha256 => Hasher::Sha256(Sha256::new()),
        }
    }

    pub fn digest(&self, content_bytes: &[u8]) -> AnyHash {
        let hash_bytes: Vec<u8> = match self {
            HashAlgorithm::Sha256 => {
                let mut d = Sha256::new();
                d.update(content_bytes);
                d.finalize().deref().into()
            }
        };

        AnyHash {
            algo: *self,
            bytes: hash_bytes,
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnyHash {
    pub(crate) algo: HashAlgorithm,
    pub(crate) bytes: Vec<u8>,
}

impl AnyHash {
    pub fn new(algo: HashAlgorithm, bytes: Vec<u8>) -> AnyHash {
        AnyHash { algo, bytes }
    }

    pub fn algorithm(&self) -> HashAlgorithm {
        self.algo
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Display for AnyHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.algo, hex::encode(self.bytes.as_slice()))
    }
}

impl fmt::Debug for AnyHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.algo, hex::encode(self.bytes.as_slice()))
    }
}

impl FromStr for AnyHash {
    type Err = AnyHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (algo_part, bytes_part) = s
            .split_once(':')
            .ok_or_else(|| AnyHashError::IncorrectStructure(s.matches(':').count() + 1))?;

        if bytes_part.chars().any(|c| "ABCDEF".contains(c)) {
            return Err(AnyHashError::UppercaseHex);
        }

        let algo = algo_part.parse::<HashAlgorithm>()?;
        let bytes = hex::decode(bytes_part)?;

        Ok(AnyHash { algo, bytes })
    }
}

#[derive(Error, Debug)]
pub enum AnyHashError {
    #[error("expected two parts for hash; found {0}")]
    IncorrectStructure(usize),

    #[error("unable to parse hash algorithm: {0}")]
    InvalidHashAlgorithm(#[from] Error),

    #[error("hash contained uppercase hex values")]
    UppercaseHex,

    #[error("hexadecimal decode failed: {0}")]
    InvalidHex(#[from] hex::FromHexError),
}

impl Serialize for AnyHash {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AnyHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::from_str(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_labeled_digest() {
        let input = b"The quick brown fox jumped over the lazy dog";
        let output = HashAlgorithm::Sha256.digest(input);
        let output = format!("{}", output);

        let expected = "sha256:7d38b5cd25a2baf85ad3bb5b9311383e671a8a142eb302b324d4a5fba8748c69";

        assert_eq!(output, expected)
    }

    #[test]
    fn test_labeled_digest_parse_rejects_uppercase() {
        let digest_str = "sha256:7d38b5cd25a2baf85ad3bb5b9311383e671a8a142eb302b324d4a5fba8748c69";
        assert!(digest_str.parse::<AnyHash>().is_ok());

        let (algo, encoded) = digest_str.split_once(':').unwrap();
        let digest_str = String::from(algo) + ":" + &encoded.to_uppercase();
        assert!(digest_str.parse::<AnyHash>().is_err());
    }

    #[test]
    fn test_labeled_digest_roundtrip() {
        let input = "sha256:7d38b5cd25a2baf85ad3bb5b9311383e671a8a142eb302b324d4a5fba8748c69";
        let output = format!("{}", input.parse::<AnyHash>().unwrap());
        assert_eq!(input, &output);
    }
}
