use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::{dsse::Signature, Error};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaintainerKey {
    pub id: String,
    pub public_key: MaintainerPublicKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "algo", content = "key")]
pub enum MaintainerPublicKey {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

pub enum MaintainerSecretKey {
    EcdsaP256(p256::ecdsa::SigningKey),
    Ed25519(ed25519_compact::SecretKey),
}

impl MaintainerSecretKey {
    pub fn generate() -> Self {
        Self::Ed25519(ed25519_compact::KeyPair::generate().sk)
    }

    pub fn public_key(&self) -> MaintainerPublicKey {
        match self {
            Self::EcdsaP256(sk) => sk.verifying_key().into(),
            Self::Ed25519(sk) => sk.public_key().into(),
        }
    }

    pub fn sign_payload(
        &self,
        payload_type: &str,
        payload: &[u8],
        key_id: String,
    ) -> Result<Signature, Error> {
        match self {
            MaintainerSecretKey::EcdsaP256(sk) => {
                Signature::sign_payload(payload_type, payload, sk, Some(key_id))
            }
            MaintainerSecretKey::Ed25519(sk) => {
                Signature::sign_payload(payload_type, payload, sk, Some(key_id))
            }
        }
    }
}
