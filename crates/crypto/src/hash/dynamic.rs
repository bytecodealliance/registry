use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Deref, str::FromStr};
use thiserror::Error;

use super::{Digest, HashAlgorithm, Sha256};

impl HashAlgorithm {
    pub fn digest(&self, content_bytes: &[u8]) -> DynHash {
        let hash_bytes: Vec<u8> = match self {
            HashAlgorithm::Sha256 => {
                let mut d = Sha256::new();
                d.update(content_bytes);
                d.finalize().deref().into()
            }
        };

        DynHash {
            algo: *self,
            bytes: hash_bytes,
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DynHash {
    pub(crate) algo: HashAlgorithm,
    pub(crate) bytes: Vec<u8>,
}

impl DynHash {
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algo
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Display for DynHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.algo, hex::encode(self.bytes.as_slice()))
    }
}

impl fmt::Debug for DynHash {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.algo, hex::encode(self.bytes.as_slice()))
    }
}

impl FromStr for DynHash {
    type Err = DynHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (algo_part, bytes_part) = s
            .split_once(':')
            .ok_or_else(|| DynHashError::IncorrectStructure(s.matches(':').count() + 1))?;

        if bytes_part.chars().any(|c| "ABCDEF".contains(c)) {
            return Err(DynHashError::UppercaseHex);
        }

        let algo = algo_part.parse::<HashAlgorithm>()?;
        let bytes = hex::decode(bytes_part)?;

        Ok(DynHash { algo, bytes })
    }
}

#[derive(Error, Debug)]
pub enum DynHashError {
    #[error("expected two parts for hash; found {0}")]
    IncorrectStructure(usize),

    #[error("unable to parse hash algorithm: {0}")]
    InvalidHashAlgorithm(#[from] Error),

    #[error("hash contained uppercase hex values")]
    UppercaseHex,

    #[error("hexadecimal decode failed: {0}")]
    InvalidHex(#[from] hex::FromHexError),
}

impl Serialize for DynHash {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for DynHash {
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
        assert!(digest_str.parse::<DynHash>().is_ok());

        let (algo, encoded) = digest_str.split_once(':').unwrap();
        let digest_str = String::from(algo) + ":" + &encoded.to_uppercase();
        assert!(digest_str.parse::<DynHash>().is_err());
    }

    #[test]
    fn test_labeled_digest_roundtrip() {
        let input = "sha256:7d38b5cd25a2baf85ad3bb5b9311383e671a8a142eb302b324d4a5fba8748c69";
        let output = format!("{}", input.parse::<DynHash>().unwrap());
        assert_eq!(input, &output);
    }
}
