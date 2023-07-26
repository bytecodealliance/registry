use serde::{Deserialize, Serialize};
use warg_crypto::{signing, Signable};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerdeEnvelope<Contents> {
    /// The content represented by content_bytes
    contents: Contents,
    /// The hash of the key that signed this envelope
    key_id: signing::KeyID,
    /// The signature for the content_bytes
    signature: signing::Signature,
}

impl<Contents> SerdeEnvelope<Contents> {
    /// Creates a new `SerdeEnvelope` from the given content, key ID, and signature.
    ///
    /// Note that this does not verify the signature matches the contents (hence unchecked).
    pub fn from_parts_unchecked(
        contents: Contents,
        key_id: signing::KeyID,
        signature: signing::Signature,
    ) -> Self {
        Self {
            contents,
            key_id,
            signature,
        }
    }

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

    pub fn into_contents(self) -> Contents {
        self.contents
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
