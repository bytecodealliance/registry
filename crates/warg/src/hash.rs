use std::{fmt, ops::Deref, str::FromStr};

use hex;
use digest::Digest as DigestTrait;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum HashAlgorithm {
    Sha256,
}

pub const SHA_256: HashAlgorithm = HashAlgorithm::Sha256;

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
            HashAlgorithm::Sha256 => write!(f, "sha256"),
        }
    }
}

impl FromStr for HashAlgorithm {
    type Err = HashAlgorithmParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sha256" => Ok(HashAlgorithm::Sha256),
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
        write!(f, "{}:{}", self.algo, hex::encode(&self.bytes))
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

        if bytes_part.chars().any(|c| "ABCDEF".contains(c)) {
            return Err(HashParseError::UppercaseHex);
        }

        let algo = algo_part.parse::<HashAlgorithm>()?;
        let bytes = hex::decode(bytes_part)?;

        Ok(Digest { algo, bytes })
    }
}

#[derive(Error, Debug)]
pub enum HashParseError {
    #[error("expected 2 parts, found {0}")]
    IncorrectStructure(usize),

    #[error("unable to parse hash algorithm")]
    HashAlgorithmParseError(#[from] HashAlgorithmParseError),

    #[error("contained uppercase hex value(s)")]
    UppercaseHex,

    #[error("hexadecimal decode failed")]
    HexDecodEError(#[from] hex::FromHexError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_digest() {
        let input = b"The quick brown fox jumped over the lazy dog";
        let output = SHA_256.digest(input);
        let output = format!("{}", output);

        let expected = "sha256:7d38b5cd25a2baf85ad3bb5b9311383e671a8a142eb302b324d4a5fba8748c69";

        assert_eq!(output, expected)
    }

    #[test]
    fn test_digest_parse_rejects_uppercase() {
        let digest_str = "sha256:7d38b5cd25a2baf85ad3bb5b9311383e671a8a142eb302b324d4a5fba8748c69";
        assert!(digest_str.parse::<Digest>().is_ok());

        let (algo, encoded) = digest_str.split_once(":").unwrap();
        let digest_str = String::from(algo) + ":" + &encoded.to_uppercase();
        assert!(digest_str.parse::<Digest>().is_err());
    }

    #[test]
    fn test_roundtrip() {
        let input = "sha256:7d38b5cd25a2baf85ad3bb5b9311383e671a8a142eb302b324d4a5fba8748c69";
        let output = format!("{}", input.parse::<Digest>().unwrap());
        assert_eq!(input, &output);
    }
}