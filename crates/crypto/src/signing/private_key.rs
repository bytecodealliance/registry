use super::{PublicKey, Signature, SignatureAlgorithm, SignatureAlgorithmParseError};
use base64::{engine::general_purpose::STANDARD, Engine};
use p256;
use secrecy::{zeroize::Zeroizing, ExposeSecret, Secret, Zeroize};
use signature::Signer;
use thiserror::Error;

pub use signature::Error as SignatureError;

/// Represents a private key
pub struct PrivateKey(Secret<PrivateKeyInner>);

pub enum PrivateKeyInner {
    EcdsaP256(p256::ecdsa::SigningKey),
}

impl PrivateKey {
    /// Decode a key from the given string in `<algo>:<base64 data>` form.
    pub fn decode(s: String) -> Result<Self, PrivateKeyParseError> {
        let s = Zeroizing::new(s);

        let Some((algo, b64_data)) = s.split_once(':') else {
            return Err(PrivateKeyParseError::MissingColon)
        };

        let algo = algo.parse::<SignatureAlgorithm>()?;
        let bytes = STANDARD.decode(b64_data)?;

        let key = match algo {
            SignatureAlgorithm::EcdsaP256 => PrivateKeyInner::EcdsaP256(
                p256::ecdsa::SigningKey::from_bytes(bytes.as_slice().into())?,
            ),
        };

        Ok(PrivateKey(Secret::from(key)))
    }

    /// Encode the key as a string in `<algo>:<base64 data>` form.
    pub fn encode(&self) -> Zeroizing<String> {
        Zeroizing::new(format!(
            "{algo}:{b64}",
            algo = self.signature_algorithm(),
            b64 = STANDARD.encode(self.bytes())
        ))
    }

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

// Note: FromStr isn't used because it makes it too easy to leave behind an
// unzeroized copy of the sensitive encoded key.
impl TryFrom<String> for PrivateKey {
    type Error = PrivateKeyParseError;

    fn try_from(key: String) -> Result<Self, PrivateKeyParseError> {
        let key = Zeroizing::new(key);

        let Some((algo, b64_data)) = key.split_once(':') else {
            return Err(PrivateKeyParseError::MissingColon)
        };

        let algo = algo.parse::<SignatureAlgorithm>()?;
        let bytes = STANDARD.decode(b64_data)?;

        let key = match algo {
            SignatureAlgorithm::EcdsaP256 => PrivateKeyInner::EcdsaP256(
                p256::ecdsa::SigningKey::from_bytes(bytes.as_slice().into())?,
            ),
        };

        Ok(PrivateKey(Secret::from(key)))
    }
}

#[derive(Error, Debug)]
pub enum PrivateKeyParseError {
    #[error("expected algorithm followed by colon")]
    MissingColon,

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
        let key = PrivateKey::decode(key_str.to_string()).unwrap();
        assert_eq!(key_str, &*key.encode());
    }
}
