use super::{Signature, SignatureAlgorithm, SignatureAlgorithmParseError};
use base64::{engine::general_purpose::STANDARD, Engine};
use core::fmt;
use serde::{Deserialize, Serialize};
use signature::{Error as SignatureError, Verifier};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicKey {
    EcdsaP256(p256::ecdsa::VerifyingKey),
}

impl PublicKey {
    /// The signature algorithm used by this key
    pub fn signature_algorithm(&self) -> SignatureAlgorithm {
        match self {
            PublicKey::EcdsaP256(_) => SignatureAlgorithm::EcdsaP256,
        }
    }

    /// Get the encoded bytes of this key
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            PublicKey::EcdsaP256(key) => key.to_encoded_point(true).as_bytes().to_vec(),
        }
    }

    /// Verify that a given message and signature were signed by the private key associated with this public key
    pub fn verify(&self, msg: &[u8], signature: &Signature) -> Result<(), SignatureError> {
        match (self, signature) {
            (PublicKey::EcdsaP256(key), Signature::P256(signature)) => key.verify(msg, signature),
        }
    }

    /// Compute the digest of this key
    pub fn fingerprint(&self) -> KeyID {
        let key_hash = self
            .signature_algorithm()
            .digest_algorithm()
            .digest(format!("{}", self).as_bytes());

        KeyID(format!("{}", key_hash))
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            self.signature_algorithm(),
            STANDARD.encode(self.bytes())
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
        let bytes = STANDARD.decode(parts[1])?;

        let key = match algo {
            SignatureAlgorithm::EcdsaP256 => {
                PublicKey::EcdsaP256(p256::ecdsa::VerifyingKey::from_sec1_bytes(&bytes)?)
            }
        };

        Ok(key)
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::from_str(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
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
        PublicKey::EcdsaP256(key)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyID(String);

impl fmt::Display for KeyID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for KeyID {
    fn from(s: String) -> Self {
        KeyID(s)
    }
}

impl From<KeyID> for String {
    fn from(id: KeyID) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_roundtrip_alice() {
        let key_str = "ecdsa-p256:A1OfZz5Y9Ny7VKPVwroCTQPAr9tmlI4U/UTYHZHA87AF";
        let pub_key: PublicKey = key_str.parse().unwrap();
        assert_eq!(key_str, &format!("{pub_key}"));
    }

    #[test]
    fn test_roundtrip_bob() {
        let key_str = "ecdsa-p256:A5qc6uBi070EBb4GihGzpx6Cm5+oZnv4dWpBhhuZVagu";
        let pub_key: PublicKey = key_str.parse().unwrap();
        assert_eq!(key_str, &format!("{pub_key}"));
    }
}
