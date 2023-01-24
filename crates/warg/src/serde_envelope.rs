use serde::{Deserialize, Serialize};

use warg_crypto::{signing, Signable};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerdeEnvelope<Contents> {
    /// The content represented by content_bytes
    contents: Contents,
    /// The hash of the key that signed this envelope
    key_id: signing::KeyID,
    /// The signature for the content_bytes
    signature: signing::Signature,
}

impl<Contents> SerdeEnvelope<Contents> {
    /// Create an envelope for some contents using a signature.
    pub fn signed_contents(
        private_key: &signing::PrivateKey,
        contents: Contents,
    ) -> Result<Self, signing::SignatureError>
    where
        Contents: Signable,
    {
        let key_id = private_key.public_key().fingerprint();
        let signature = contents.sign(private_key)?;
        Ok(SerdeEnvelope {
            contents,
            key_id,
            signature,
        })
    }

    pub fn key_id(&self) -> &signing::KeyID {
        &self.key_id
    }

    pub fn signature(&self) -> &signing::Signature {
        &self.signature
    }
}

impl<Content> AsRef<Content> for SerdeEnvelope<Content> {
    fn as_ref(&self) -> &Content {
        &self.contents
    }
}
