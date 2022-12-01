use prost::Message;
use std::error::Error;
use thiserror::Error;

use crate::hash::{self, HashParseError};
use crate::signing::{self, SignatureParseError};
use signature::Error as SignatureError;

pub mod model;
pub mod validate;

/// The protobuf encoding of the package types
pub mod protobuf {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/warg.package.rs"));
}

/// The envelope struct is used to keep around the original
/// bytes that the content was serialized into in case
/// the serialization is not canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope<Contents> {
    /// The content represented by content_bytes
    pub contents: Contents,
    /// The serialized representation of the content
    pub content_bytes: Vec<u8>,
    /// The hash of the key that signed this envelope
    pub key_id: signing::KeyID,
    /// The signature for the content_bytes
    pub signature: signing::Signature,
}

impl<Contents> Envelope<Contents> {
    /// Create an envelope for some contents using a signature.
    pub fn signed_contents(
        private_key: signing::PrivateKey,
        contents: Contents,
    ) -> Result<Self, SignatureError>
    where
        Contents: Signable,
    {
        let content_bytes: Vec<u8> = contents.encode();

        let key_id = private_key.public_key().fingerprint();
        let signature = contents.signature(private_key)?;
        Ok(Envelope {
            contents,
            content_bytes,
            key_id,
            signature,
        })
    }

    /// Get the representation of the entire envelope as a byte vector.
    /// This is the logical inverse of `Envelope::from_bytes`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let proto_envelope = protobuf::Envelope {
            contents: self.content_bytes.clone(),
            key_id: self.key_id.to_string(),
            signature: self.signature.to_string(),
        };
        proto_envelope.encode_to_vec()
    }

    /// Create an entire envelope from a byte vector.
    /// This is the logical inverse of `Envelope::as_bytes`.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ParseEnvelopeError>
    where
        Contents: for<'a> TryFrom<&'a [u8], Error = ErrorBox>,
    {
        // Parse outer envelope
        let envelope = protobuf::Envelope::decode(&*bytes)?;
        // Parse contents
        let content_bytes = envelope.contents.clone();
        let contents = content_bytes
            .as_slice()
            .try_into()
            .map_err(ParseEnvelopeError::ContentsParseError)?;
        // Read key ID and signature
        let key_id = envelope.key_id.into();
        let signature = envelope.signature.parse()?;

        Ok(Envelope {
            contents,
            content_bytes,
            key_id,
            signature,
        })
    }
}

type ErrorBox = Box<dyn Error + Send + Sync + 'static>;

/// Errors that occur in the process of parsing an envelope from bytes
#[derive(Error, Debug)]
pub enum ParseEnvelopeError {
    #[error("Failed to parse the outer envelope protobuf message")]
    ProtobufEnvelopeParseError(#[from] prost::DecodeError),

    #[error("Failed to parse envelope contents from bytes")]
    ContentsParseError(ErrorBox),

    #[error("Failed to parse envelope key id")]
    KeyIDParseError(#[from] HashParseError),

    #[error("Failed to parse envelope signature")]
    SignatureParseError(#[from] SignatureParseError),
}

pub trait Signable: Encode {
    const PREFIX: &'static [u8];

    fn signature(
        &self,
        private_key: signing::PrivateKey,
    ) -> Result<signing::Signature, SignatureError> {
        let prefixed_content = [Self::PREFIX, self.encode().as_slice()].concat();
        private_key.sign(&prefixed_content)
    }

    fn verify(
        public_key: signing::PublicKey,
        msg: &[u8],
        signature: &signing::Signature,
    ) -> Result<(), SignatureError> {
        let prefixed_content = [Self::PREFIX, msg].concat();
        public_key.verify(&prefixed_content, signature)
    }
}

pub trait Encode {
    fn encode(&self) -> Vec<u8>;
}

// Deserialization

impl TryFrom<&[u8]> for model::PackageRecord {
    type Error = ErrorBox;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        protobuf::PackageRecord::decode(bytes)?.try_into()
    }
}

impl TryFrom<protobuf::PackageRecord> for model::PackageRecord {
    type Error = ErrorBox;

    fn try_from(record: protobuf::PackageRecord) -> Result<Self, Self::Error> {
        let prev: Option<hash::Digest> = match record.prev {
            Some(hash_string) => Some(hash_string.parse()?),
            None => None,
        };
        let version = record.version;
        let timestamp = record
            .time
            .ok_or_else(|| Box::new(InvalidTimestampError))?
            .try_into()?;
        let entries: Result<Vec<model::PackageEntry>, ErrorBox> = record
            .entries
            .into_iter()
            .map(|proto_entry| proto_entry.try_into())
            .collect();
        let entries = entries?;

        Ok(model::PackageRecord {
            prev,
            version,
            timestamp,
            entries,
        })
    }
}

#[derive(Error, Debug)]
#[error("Empty or invalid timestamp in record")]
struct InvalidTimestampError;

impl TryFrom<protobuf::PackageEntry> for model::PackageEntry {
    type Error = ErrorBox;

    fn try_from(entry: protobuf::PackageEntry) -> Result<Self, Self::Error> {
        use protobuf::package_entry::Contents;
        let output = match entry.contents.ok_or_else(|| Box::new(EmptyContentError))? {
            Contents::Init(init) => model::PackageEntry::Init {
                hash_algorithm: init.hash_algorithm.parse()?,
                key: init.key.parse()?,
            },
            Contents::GrantFlat(grant_flat) => model::PackageEntry::GrantFlat {
                key: grant_flat.key.parse()?,
                permission: grant_flat.permission.try_into()?,
            },
            Contents::RevokeFlat(revoke_flat) => model::PackageEntry::RevokeFlat {
                key_id: revoke_flat.key_id.into(),
                permission: revoke_flat.permission.try_into()?,
            },
            Contents::Release(release) => model::PackageEntry::Release {
                version: release
                    .version
                    .parse()
                    .map_err(|error| Box::new(error) as ErrorBox)?,
                content: release.content_hash.parse()?,
            },
            Contents::Yank(yank) => model::PackageEntry::Yank {
                version: yank.version.parse()?,
            },
        };
        Ok(output)
    }
}

#[derive(Error, Debug)]
#[error("No content in entry")]
struct EmptyContentError;

impl TryFrom<i32> for model::Permission {
    type Error = ErrorBox;

    fn try_from(permission: i32) -> Result<Self, Self::Error> {
        let proto_perm = protobuf::Permission::from_i32(permission)
            .ok_or_else(|| Box::new(PermissionParseError { value: permission }))?;
        match proto_perm {
            protobuf::Permission::Release => Ok(model::Permission::Release),
            protobuf::Permission::Yank => Ok(model::Permission::Yank),
        }
    }
}

#[derive(Error, Debug)]
#[error("The value {value} could not be parsed as a permission")]
struct PermissionParseError {
    value: i32,
}

// Serialization

impl Signable for model::PackageRecord {
    const PREFIX: &'static [u8] = b"WARG-PACKAGE-RECORD-SIGNATURE-V0:";
}

impl Encode for model::PackageRecord {
    fn encode(&self) -> Vec<u8> {
        let proto_record: protobuf::PackageRecord = self.into();
        proto_record.encode_to_vec()
    }
}

impl<'a> From<&'a model::PackageRecord> for protobuf::PackageRecord {
    fn from(record: &'a model::PackageRecord) -> Self {
        protobuf::PackageRecord {
            prev: record.prev.as_ref().map(|hash| hash.to_string()),
            version: record.version,
            time: Some(record.timestamp.into()),
            entries: record.entries.iter().map(|entry| entry.into()).collect(),
        }
    }
}

impl<'a> From<&'a model::PackageEntry> for protobuf::PackageEntry {
    fn from(entry: &'a model::PackageEntry) -> Self {
        use protobuf::package_entry::Contents;
        let contents = match entry {
            model::PackageEntry::Init {
                hash_algorithm,
                key,
            } => Contents::Init(protobuf::Init {
                key: key.to_string(),
                hash_algorithm: hash_algorithm.to_string(),
            }),
            model::PackageEntry::GrantFlat { key, permission } => {
                Contents::GrantFlat(protobuf::GrantFlat {
                    key: key.to_string(),
                    permission: permission.into(),
                })
            }
            model::PackageEntry::RevokeFlat { key_id, permission } => {
                Contents::RevokeFlat(protobuf::RevokeFlat {
                    key_id: key_id.to_string(),
                    permission: permission.into(),
                })
            }
            model::PackageEntry::Release { version, content } => {
                Contents::Release(protobuf::Release {
                    version: version.to_string(),
                    content_hash: content.to_string(),
                })
            }
            model::PackageEntry::Yank { version } => Contents::Yank(protobuf::Yank {
                version: version.to_string(),
            }),
        };
        let contents = Some(contents);
        protobuf::PackageEntry { contents }
    }
}

impl<'a> From<&'a model::Permission> for i32 {
    fn from(permission: &'a model::Permission) -> Self {
        let proto_perm = match permission {
            model::Permission::Release => protobuf::Permission::Release,
            model::Permission::Yank => protobuf::Permission::Yank,
        };
        proto_perm.into()
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    use crate::hash::HashAlgorithm;
    use crate::signing::tests::generate_p256_pair;
    use semver::Version;

    #[test]
    fn test_envelope_roundtrip() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let (bob_pub, _bob_priv) = generate_p256_pair();

        let record = model::PackageRecord {
            prev: None,
            version: 0,
            timestamp: SystemTime::now(),
            entries: vec![
                model::PackageEntry::Init {
                    hash_algorithm: HashAlgorithm::Sha256,
                    key: alice_pub,
                },
                model::PackageEntry::GrantFlat {
                    key: bob_pub.clone(),
                    permission: model::Permission::Release,
                },
                model::PackageEntry::RevokeFlat {
                    key_id: bob_pub.fingerprint(),
                    permission: model::Permission::Release,
                },
                model::PackageEntry::Release {
                    version: Version::new(1, 0, 0),
                    content: HashAlgorithm::Sha256.digest(&[0, 1, 2, 3]),
                },
            ],
        };

        let first_envelope = match Envelope::signed_contents(alice_priv, record) {
            Ok(value) => value,
            Err(error) => panic!("Failed to sign envelope 1: {:?}", error),
        };

        let bytes = first_envelope.to_bytes();

        let second_envelope: Envelope<model::PackageRecord> = match Envelope::from_bytes(bytes) {
            Ok(value) => value,
            Err(error) => panic!("Failed to create envelope 2: {:?}", error),
        };

        assert_eq!(first_envelope, second_envelope);
    }
}
