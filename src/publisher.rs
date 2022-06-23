use ed25519_compact::{Error, KeyPair, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrototypePublisher {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(with = "crate::serde::base64")]
    pub ed25519_pubkey: Vec<u8>,
}

impl PrototypePublisher {
    pub fn generate() -> (Self, SecretKey) {
        let pair = KeyPair::generate();
        (
            Self {
                id: None,
                ed25519_pubkey: Vec::from(*pair.pk),
            },
            pair.sk,
        )
    }

    pub fn public_key(&self) -> Result<PublicKey, Error> {
        PublicKey::from_slice(&self.ed25519_pubkey)
    }
}
