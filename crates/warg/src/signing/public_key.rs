use base64;
use p256;
use signature::{Error as SignatureError, Verifier};

use core::fmt;
use std::str::FromStr;
use thiserror::Error;

use crate::hash;

use super::{Signature, SignatureAlgorithm, SignatureAlgorithmParseError};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicKey {
    P256(p256::ecdsa::VerifyingKey),
}

impl PublicKey {
    pub fn signature_algorithm(&self) -> SignatureAlgorithm {
        match self {
            PublicKey::P256(_) => SignatureAlgorithm::EcdsaP256,
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        match self {
            PublicKey::P256(key) => key.to_encoded_point(true).as_bytes().to_vec(),
        }
    }

    pub fn verify(&self, msg: &[u8], signature: Signature) -> Result<(), SignatureError> {
        match (self, signature) {
            (PublicKey::P256(key), Signature::P256(signature)) => key.verify(msg, &signature),
        }
    }

    pub fn digest(&self) -> hash::Hash {
        self.signature_algorithm()
            .digest_algorithm()
            .digest(format!("{}", self).as_bytes())
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            self.signature_algorithm(),
            base64::encode(&self.bytes())
        )
    }
}

impl FromStr for PublicKey {
    type Err = PublicKeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(|c| c == ':').collect();
        if parts.len() != 2 {
            return Err(PublicKeyParseError::IncorrectStructure(parts.len()));
        }
        let algo = parts[0].parse::<SignatureAlgorithm>()?;
        let bytes = base64::decode(parts[1])?;

        let key = match algo {
            SignatureAlgorithm::EcdsaP256 => {
                PublicKey::P256(p256::ecdsa::VerifyingKey::from_sec1_bytes(&bytes)?)
            }
        };

        Ok(key)
    }
}

#[derive(Error, Debug)]
pub enum PublicKeyParseError {
    #[error("expected 2 parts, found {0}")]
    IncorrectStructure(usize),

    #[error("unable to parse signature algorithm")]
    SignatureAlgorithmParseError(#[from] SignatureAlgorithmParseError),

    #[error("base64 decode failed")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("public key could not be constructed from bytes")]
    SignatureError(#[from] SignatureError),
}

impl From<p256::ecdsa::VerifyingKey> for PublicKey {
    fn from(key: p256::ecdsa::VerifyingKey) -> Self {
        PublicKey::P256(key)
    }
}
