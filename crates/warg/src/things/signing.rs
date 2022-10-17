use core::fmt;
use std::str::FromStr;

use base64::{decode, encode};

use super::hash;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Algorithm {
    ES256,
}

impl Algorithm {
    fn associated_hash_algo(&self) -> hash::Algorithm {
        match self {
            Algorithm::ES256 =>  hash::Algorithm::SHA256,
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Algorithm::ES256 => write!(f, "ES256"),
        }
    }
}

impl FromStr for Algorithm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ES256" => Ok(Algorithm::ES256),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Key {
    pub algo: Algorithm,
    pub bytes: Vec<u8>,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algo, encode(&self.bytes))
    }
}

impl FromStr for Key {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(|c| c == ':').collect();
        if parts.len() != 2 {
            return Err(());
        }
        let algo = parts[0].parse::<Algorithm>()?;
        let bytes = decode(parts[1]).map_err(|_| ())?;

        Ok(Key { algo, bytes })
    }
}

pub struct Signature {
    algo: Algorithm,
    bytes: Vec<u8>,
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algo, encode(&self.bytes))
    }
}

impl FromStr for Signature {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(|c| c == ':').collect();
        if parts.len() != 2 {
            return Err(());
        }
        let algo = parts[0].parse::<Algorithm>()?;
        let bytes = decode(parts[1]).map_err(|_| ())?;

        Ok(Signature { algo, bytes })
    }
}
