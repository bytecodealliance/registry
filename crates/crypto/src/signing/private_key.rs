use super::{PublicKey, Signature, SignatureAlgorithm, SignatureAlgorithmParseError};
use base64;
use core::fmt;
use p256;
use secrecy::{ExposeSecret, Secret, Zeroize};
use signature::Signer;
use std::str::FromStr;
use thiserror::Error;

pub use signature::Error as SignatureError;

/// Represents a private key
pub struct PrivateKey(Secret<PrivateKeyInner>);

pub enum PrivateKeyInner {
    EcdsaP256(p256::ecdsa::SigningKey),
}

impl PrivateKey {
    /// Get the signature algorithm used for by this key
    pub fn signature_algorithm(&self) -> SignatureAlgorithm {
        match self.0.expose_secret() {
            PrivateKeyInner::EcdsaP256(_) => SignatureAlgorithm::EcdsaP256,
        }
    }

    /// Get the keys representation as bytes (not including an algorithm specifier)
    pub fn bytes(&self) -> Vec<u8> {
        match self.0.expose_secret() {
            PrivateKeyInner::EcdsaP256(key) => key.to_bytes().to_vec(),
        }
    }

    /// Sign a given message with this key
    pub fn sign(&self, msg: &[u8]) -> Result<Signature, SignatureError> {
        match self.0.expose_secret() {
            PrivateKeyInner::EcdsaP256(key) => Ok(Signature::P256(key.try_sign(msg)?)),
        }
    }

    pub fn public_key(&self) -> PublicKey {
        match self.0.expose_secret() {
            PrivateKeyInner::EcdsaP256(key) => {
                PublicKey::EcdsaP256(p256::ecdsa::VerifyingKey::from(key))
            }
        }
    }
}

impl fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            self.signature_algorithm(),
            base64::encode(self.bytes())
        )
    }
}

impl FromStr for PrivateKey {
    type Err = PrivateKeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(|c| c == ':').collect();
        if parts.len() != 2 {
            return Err(PrivateKeyParseError::IncorrectStructure(parts.len()));
        }
        let algo = parts[0].parse::<SignatureAlgorithm>()?;
        let bytes = base64::decode(parts[1])?;

        let key = match algo {
            SignatureAlgorithm::EcdsaP256 => {
                PrivateKeyInner::EcdsaP256(p256::ecdsa::SigningKey::from_bytes(&bytes)?)
            }
        };

        Ok(PrivateKey(Secret::from(key)))
    }
}

#[derive(Error, Debug)]
pub enum PrivateKeyParseError {
    #[error("expected 2 parts, found {0}")]
    IncorrectStructure(usize),

    #[error("unable to parse signature algorithm")]
    SignatureAlgorithmParseError(#[from] SignatureAlgorithmParseError),

    #[error("base64 decode failed")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("private key could not be constructed from bytes")]
    SignatureError(#[from] SignatureError),
}

impl Zeroize for PrivateKeyInner {
    fn zeroize(&mut self) {
        match self {
            PrivateKeyInner::EcdsaP256(sk) => {
                // SigningKey zeroizes on Drop:
                // https://github.com/RustCrypto/signatures/blob/a97a358f9e00773c4a04ca54816fb539506f89e6/ecdsa/src/sign.rs#L118
                let mostly_zero = p256::ecdsa::SigningKey::from(
                    p256::NonZeroScalar::new(p256::Scalar::ONE).unwrap(),
                );
                drop(std::mem::replace(sk, mostly_zero));
            }
        }
    }
}

impl From<p256::ecdsa::SigningKey> for PrivateKey {
    fn from(key: p256::ecdsa::SigningKey) -> Self {
        PrivateKey(Secret::from(PrivateKeyInner::EcdsaP256(key)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_roundtrip() {
        let key_str = "ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk=";
        let pub_key: PrivateKey = key_str.parse().unwrap();
        assert_eq!(key_str, &format!("{pub_key}"));
    }
}
