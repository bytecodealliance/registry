use std::{fmt, str::FromStr, ops::Deref};

use base64::{decode, encode};
use digest::Digest;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Algorithm {
    SHA256,
}

impl Algorithm {
    pub fn digest(&self, content_bytes: &[u8]) -> Hash {
        let hash_bytes: Vec<u8> = match self {
            Algorithm::SHA256 => {
                let mut d = sha2::Sha256::new();
                d.update(content_bytes);
                d.finalize().deref().into()
            },
        };

        Hash {
            algo: *self,
            bytes: hash_bytes,
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Algorithm::SHA256 => write!(f, "SHA256"),
        }
    }
}

impl FromStr for Algorithm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SHA256" => Ok(Algorithm::SHA256),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Hash {
    algo: Algorithm,
    bytes: Vec<u8>,
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algo, encode(&self.bytes))
    }
}

impl FromStr for Hash {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(|c| c == ':').collect();
        if parts.len() != 2 {
            return Err(());
        }
        let algo = parts[0].parse::<Algorithm>()?;
        let bytes = decode(parts[1]).map_err(|_| ())?;

        Ok(Hash { algo, bytes })
    }
}
