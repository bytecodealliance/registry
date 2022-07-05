use std::borrow::Cow;

use secrecy::{ExposeSecret, Secret, Zeroize};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::{dsse::Signature, Error};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaintainerKey {
    pub id: String,
    pub public_key: MaintainerPublicKey,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "algo", content = "key")]
pub enum MaintainerPublicKey {
    // TODO: in-depth crypto lib eval
    #[serde(rename = "ecdsa-p256")]
    EcdsaP256(EcdsaP256PublicKey),
    #[serde(rename = "ed25519")]
    Ed25519(Ed25519PublicKey),
}

impl MaintainerPublicKey {
    pub(crate) fn fingerprint(&self) -> Vec<u8> {
        let bytes: Cow<[u8]> = match self {
            MaintainerPublicKey::EcdsaP256(pk) => pk.to_bytes().into(),
            MaintainerPublicKey::Ed25519(pk) => pk.as_bytes().into(),
        };
        sha2::Sha256::digest(bytes).to_vec()
    }

    pub(crate) fn verify_payload(
        &self,
        payload_type: &str,
        payload: &[u8],
        signature: &Signature,
    ) -> Result<(), Error> {
        match self {
            MaintainerPublicKey::EcdsaP256(pk) => {
                signature.verify_payload(payload_type, payload, pk.0)?;
            }
            MaintainerPublicKey::Ed25519(pk) => {
                signature.verify_payload(payload_type, payload, pk.0)?;
            }
        };
        Ok(())
    }
}

impl From<p256::ecdsa::VerifyingKey> for MaintainerPublicKey {
    fn from(key: p256::ecdsa::VerifyingKey) -> Self {
        Self::EcdsaP256(EcdsaP256PublicKey(key))
    }
}

impl From<ed25519_compact::PublicKey> for MaintainerPublicKey {
    fn from(key: ed25519_compact::PublicKey) -> Self {
        Self::Ed25519(Ed25519PublicKey(key))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "&str")]
pub struct EcdsaP256PublicKey(p256::ecdsa::VerifyingKey);

impl EcdsaP256PublicKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_encoded_point(true).as_bytes().to_vec()
    }
}

impl From<EcdsaP256PublicKey> for String {
    fn from(pk: EcdsaP256PublicKey) -> Self {
        base64::encode(pk.0.to_encoded_point(true))
    }
}

impl TryFrom<&str> for EcdsaP256PublicKey {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = base64::decode(value).map_err(|err| {
            Error::InvalidSignatureKey(format!("base64 decoding failed: {}", err).into())
        })?;
        let pk = p256::ecdsa::VerifyingKey::from_sec1_bytes(&bytes)
            .map_err(|_| Error::InvalidSignatureKey("invalid key format".into()))?;
        Ok(Self(pk))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "&str")]
pub struct Ed25519PublicKey(ed25519_compact::PublicKey);

impl Ed25519PublicKey {
    fn as_bytes(&self) -> &[u8] {
        &*self.0
    }
}

impl From<Ed25519PublicKey> for String {
    fn from(pk: Ed25519PublicKey) -> Self {
        base64::encode(*pk.0)
    }
}

impl TryFrom<&str> for Ed25519PublicKey {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = base64::decode(value).map_err(|err| {
            Error::InvalidSignatureKey(format!("base64 decoding failed: {}", err).into())
        })?;
        let pk = ed25519_compact::PublicKey::from_slice(&bytes)
            .map_err(|_| Error::InvalidSignatureKey("invalid key format".into()))?;
        Ok(Self(pk))
    }
}

pub struct MaintainerSecret {
    pub key_id: String,
    // Secret helps prevent accidental key leakage.
    pub secret_key: Secret<MaintainerSecretKey>,
}

impl MaintainerSecret {
    pub fn new(key_id: String, secret_key: MaintainerSecretKey) -> Self {
        let key = Secret::new(secret_key);
        Self {
            key_id,
            secret_key: key,
        }
    }

    pub fn generate() -> Self {
        let secret_key = MaintainerSecretKey::Ed25519(ed25519_compact::KeyPair::generate().sk);
        Self::new(String::new(), secret_key)
    }

    pub fn public_key(&self) -> MaintainerPublicKey {
        match self.secret_key.expose_secret() {
            MaintainerSecretKey::EcdsaP256(sk) => sk.verifying_key().into(),
            MaintainerSecretKey::Ed25519(sk) => sk.public_key().into(),
        }
    }

    pub fn sign_payload(&self, payload_type: &str, payload: &[u8]) -> Result<Signature, Error> {
        let key_id = self.key_id.clone();
        match self.secret_key.expose_secret() {
            MaintainerSecretKey::EcdsaP256(sk) => {
                Signature::sign_payload(payload_type, payload, sk, Some(key_id))
            }
            MaintainerSecretKey::Ed25519(sk) => {
                Signature::sign_payload(payload_type, payload, sk, Some(key_id))
            }
        }
    }
}

pub enum MaintainerSecretKey {
    EcdsaP256(p256::ecdsa::SigningKey),
    Ed25519(ed25519_compact::SecretKey),
}

impl Zeroize for MaintainerSecretKey {
    fn zeroize(&mut self) {
        match self {
            MaintainerSecretKey::EcdsaP256(sk) => {
                // SigningKey zeroizes on Drop:
                // https://github.com/RustCrypto/signatures/blob/a97a358f9e00773c4a04ca54816fb539506f89e6/ecdsa/src/sign.rs#L118
                let mostly_zero = p256::ecdsa::SigningKey::from(
                    p256::NonZeroScalar::new(p256::Scalar::ONE).unwrap(),
                );
                drop(std::mem::replace(sk, mostly_zero));
            }
            MaintainerSecretKey::Ed25519(_sk) => {
                // FIXME(lann): Implement after release of https://github.com/jedisct1/rust-ed25519-compact/commit/14669deee0b0dc6e6db189f66fcda4585ce1e82f
            }
        }
    }
}
