use anyhow::Error;
use base64::{engine::general_purpose::STANDARD, Engine};
use prost::Message;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use std::fmt;
use thiserror::Error;
use warg_crypto::{hash::AnyHashError, signing, Decode, Signable};
use warg_protobuf::protocol as protobuf;

/// The envelope struct is used to keep around the original
/// bytes that the content was serialized into in case
/// the serialization is not canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtoEnvelope<Contents> {
    /// The content represented by content_bytes
    contents: Contents,
    /// The serialized representation of the content
    content_bytes: Vec<u8>,
    /// The hash of the key that signed this envelope
    key_id: signing::KeyID,
    /// The signature for the content_bytes
    signature: signing::Signature,
}

impl<Contents> ProtoEnvelope<Contents> {
    /// Create an envelope for some contents using a signature.
    pub fn signed_contents(
        private_key: &signing::PrivateKey,
        contents: Contents,
    ) -> Result<Self, signing::SignatureError>
    where
        Contents: Signable,
    {
        let content_bytes: Vec<u8> = contents.encode();

        let key_id = private_key.public_key().fingerprint();
        let signature = contents.sign(private_key)?;
        Ok(ProtoEnvelope {
            contents,
            content_bytes,
            key_id,
            signature,
        })
    }

    /// Get the byte representation of the envelope contents.
    pub fn content_bytes(&self) -> &[u8] {
        &self.content_bytes
    }

    pub fn key_id(&self) -> &signing::KeyID {
        &self.key_id
    }

    pub fn signature(&self) -> &signing::Signature {
        &self.signature
    }

    /// Get the representation of the entire envelope as a byte vector.
    /// This is the logical inverse of `Envelope::from_bytes`.
    pub fn to_protobuf(&self) -> Vec<u8> {
        let proto_envelope = protobuf::Envelope {
            contents: self.content_bytes.clone(),
            key_id: self.key_id.to_string(),
            signature: self.signature.to_string(),
        };
        proto_envelope.encode_to_vec()
    }

    /// Create an entire envelope from a byte vector.
    /// This is the logical inverse of `Envelope::as_bytes`.
    pub fn from_protobuf(bytes: Vec<u8>) -> Result<Self, ParseEnvelopeError>
    where
        Contents: Decode,
    {
        // Parse outer envelope
        let envelope = protobuf::Envelope::decode(bytes.as_slice())?;
        let contents = Contents::decode(&envelope.contents)?;

        // Read key ID and signature
        let key_id = envelope.key_id.into();
        let signature = envelope.signature.parse()?;

        Ok(ProtoEnvelope {
            contents,
            content_bytes: envelope.contents,
            key_id,
            signature,
        })
    }
}

impl<Content> AsRef<Content> for ProtoEnvelope<Content> {
    fn as_ref(&self) -> &Content {
        &self.contents
    }
}

/// Errors that occur in the process of parsing an envelope from bytes
#[derive(Error, Debug)]
pub enum ParseEnvelopeError {
    #[error("failed to parse the outer envelope protobuf message")]
    ProtobufEnvelope(#[from] prost::DecodeError),

    #[error("failed to parse envelope contents from bytes")]
    Contents(#[from] Error),

    #[error("failed to parse envelope key id")]
    KeyID(#[from] AnyHashError),

    #[error("failed to parse envelope signature")]
    Signature(#[from] signing::SignatureParseError),
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtoEnvelopeBody {
    /// The serialized representation of the content
    #[serde_as(as = "Base64")]
    content_bytes: Vec<u8>,
    /// The hash of the key that signed this envelope
    key_id: signing::KeyID,
    /// The signature for the content_bytes
    signature: signing::Signature,
}

impl<Content> TryFrom<ProtoEnvelopeBody> for ProtoEnvelope<Content>
where
    Content: Decode,
{
    type Error = Error;

    fn try_from(value: ProtoEnvelopeBody) -> Result<Self, Self::Error> {
        let contents = Content::decode(&value.content_bytes)?;
        let envelope = ProtoEnvelope {
            contents,
            content_bytes: value.content_bytes,
            key_id: value.key_id,
            signature: value.signature,
        };
        Ok(envelope)
    }
}

impl<Content> From<ProtoEnvelope<Content>> for ProtoEnvelopeBody {
    fn from(value: ProtoEnvelope<Content>) -> Self {
        ProtoEnvelopeBody {
            content_bytes: value.content_bytes,
            key_id: value.key_id,
            signature: value.signature,
        }
    }
}

impl fmt::Debug for ProtoEnvelopeBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProtoEnvelopeBody")
            .field("content_bytes", &STANDARD.encode(&self.content_bytes))
            .field("key_id", &self.key_id)
            .field("signature", &self.signature)
            .finish()
    }
}
