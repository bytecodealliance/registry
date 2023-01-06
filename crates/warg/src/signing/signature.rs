use super::{SignatureAlgorithm, SignatureAlgorithmParseError};
use base64;
use core::fmt;
use p256;
use serde::{Deserialize, Serialize};
use signature::Error as SignatureError;
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signature {
    P256(p256::ecdsa::Signature),
}

impl Signature {
    /// Get the signature algorithm used to create this signature
    pub fn signature_algorithm(&self) -> SignatureAlgorithm {
        match self {
            Signature::P256(_) => SignatureAlgorithm::EcdsaP256,
        }
    }

    /// Get the signature's representation as bytes (not including an algorithm specifier)
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            Signature::P256(key) => key.to_der().to_bytes().to_vec(),
        }
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            self.signature_algorithm(),
            base64::encode(self.bytes())
        )
    }
}

impl FromStr for Signature {
    type Err = SignatureParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(|c| c == ':').collect();
        if parts.len() != 2 {
            return Err(SignatureParseError::IncorrectStructure(parts.len()));
        }
        let algo = parts[0].parse::<SignatureAlgorithm>()?;
        let bytes = base64::decode(parts[1])?;

        let sig = match algo {
            SignatureAlgorithm::EcdsaP256 => {
                Signature::P256(p256::ecdsa::Signature::from_der(&bytes)?)
            }
        };

        Ok(sig)
    }
}

impl Serialize for Signature {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::from_str(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Error, Debug)]
pub enum SignatureParseError {
    #[error("expected 2 parts, found {0}")]
    IncorrectStructure(usize),

    #[error("unable to parse signature algorithm")]
    SignatureAlgorithmParseError(#[from] SignatureAlgorithmParseError),

    #[error("base64 decode failed")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("signature could not be constructed from bytes")]
    SignatureError(#[from] SignatureError),
}
