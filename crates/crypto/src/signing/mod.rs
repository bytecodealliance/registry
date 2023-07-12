use core::fmt;
use rand_core::OsRng;
use std::str::FromStr;
use thiserror::Error;

use crate::hash::HashAlgorithm;

mod private_key;
mod public_key;
pub mod signature;

pub use self::private_key::{PrivateKey, PrivateKeyParseError, SignatureError};
pub use self::public_key::{KeyID, PublicKey, PublicKeyParseError};
pub use self::signature::{Signature, SignatureParseError};

/// A signature algorithm supported by WARG
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    EcdsaP256,
}

impl SignatureAlgorithm {
    /// Determine which hash algorithm is used by this
    /// signing algorithm to generate digests.
    pub fn digest_algorithm(&self) -> HashAlgorithm {
        match self {
            SignatureAlgorithm::EcdsaP256 => HashAlgorithm::Sha256,
        }
    }
}

impl fmt::Display for SignatureAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignatureAlgorithm::EcdsaP256 => write!(f, "ecdsa-p256"),
        }
    }
}

impl FromStr for SignatureAlgorithm {
    type Err = SignatureAlgorithmParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ecdsa-p256" => Ok(SignatureAlgorithm::EcdsaP256),
            _ => Err(SignatureAlgorithmParseError {
                value: s.to_owned(),
            }),
        }
    }
}

#[derive(Error, Debug)]
#[error("\"{value}\" is not a valid algorithm choice")]
pub struct SignatureAlgorithmParseError {
    value: String,
}

pub fn generate_p256_pair() -> (PublicKey, PrivateKey) {
    let private_key = p256::ecdsa::SigningKey::random(&mut OsRng);
    let public_key = p256::ecdsa::VerifyingKey::from(&private_key);
    (PublicKey::from(public_key), PrivateKey::from(private_key))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn test_correct_key_passes_verify() {
        let (public, private) = generate_p256_pair();
        let msg = (0..255u8).collect::<Vec<u8>>();
        let signature = private.sign(&msg).unwrap();
        public.verify(&msg, &signature).unwrap();
    }

    #[test]
    pub fn test_wrong_key_fails_verify() {
        let (alice_public, alice_private) = generate_p256_pair();
        let (bob_public, bob_private) = generate_p256_pair();

        let msg = (0..255u8).collect::<Vec<u8>>();
        let alice_signature = alice_private.sign(&msg).unwrap();
        let bob_signature = bob_private.sign(&msg).unwrap();

        assert!(bob_public.verify(&msg, &alice_signature).is_err());
        assert!(alice_public.verify(&msg, &bob_signature).is_err());
    }
}
