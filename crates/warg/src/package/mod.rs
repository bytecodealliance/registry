use prost::Message;
use thiserror::Error;

use crate::hash::{self, HashParseError};
use crate::signing::{self, SignatureParseError};
use signature::Error as SignatureError;

pub mod model;
pub mod validate;

pub mod protobuf {
    include!(concat!(env!("OUT_DIR"), "/warg.package.rs"));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope<Contents> {
    pub contents: Contents,
    pub content_bytes: Vec<u8>,
    pub key_id: hash::Hash,
    pub signature: signing::Signature,
}

impl<Contents> Envelope<Contents> {
    pub fn signed_contents(
        private_key: signing::PrivateKey,
        contents: Contents,
    ) -> Result<Self, SignatureError>
    where
        Contents: Into<Vec<u8>> + Clone,
    {
        let content_bytes: Vec<u8> = contents.clone().into();

        let key_id = private_key.public_key().digest();
        let signature = private_key.sign(&content_bytes)?;
        Ok(Envelope {
            contents,
            content_bytes,
            key_id,
            signature,
        })
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let proto_envelope = protobuf::Envelope {
            contents: self.content_bytes.clone(),
            key_id: self.key_id.to_string(),
            signature: self.signature.to_string(),
        };
        proto_envelope.encode_to_vec()
    }

    pub fn from_bytes<ContentsParseError>(
        bytes: Vec<u8>,
    ) -> Result<Self, ParseEnvelopeError<ContentsParseError>>
    where
        Contents: for<'a> TryFrom<&'a [u8], Error = ContentsParseError>,
    {
        // Parse outer envelope
        let envelope = protobuf::Envelope::decode(&*bytes)?;
        // Parse contents
        let content_bytes = envelope.contents.clone();
        let contents = content_bytes
            .as_slice()
            .try_into()
            .map_err(|error| ParseEnvelopeError::ContentsParseError(error))?;
        // Read key ID and signature
        let key_id = envelope.key_id.parse()?;
        let signature = envelope.signature.parse()?;

        Ok(Envelope {
            contents,
            content_bytes,
            key_id,
            signature,
        })
    }
}

#[derive(Error, Debug)]
pub enum ParseEnvelopeError<ContentsParseError> {
    #[error("Failed to parse the outer envelope protobuf message")]
    ProtobufEnvelopeParseError(#[from] prost::DecodeError),

    #[error("Failed to parse envelope contents from bytes")]
    ContentsParseError(ContentsParseError),

    #[error("Failed to parse envelope key id")]
    KeyIDParseError(#[from] HashParseError),

    #[error("Failed to parse envelope signature")]
    SignatureParseError(#[from] SignatureParseError),
}

// Deserialization

impl TryFrom<&[u8]> for model::Record {
    type Error = ();

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        protobuf::PackageRecord::decode(bytes)
            .map_err(|_| ())?
            .try_into()
    }
}

impl TryFrom<protobuf::PackageRecord> for model::Record {
    type Error = ();

    fn try_from(record: protobuf::PackageRecord) -> Result<Self, Self::Error> {
        let prev: Option<hash::Hash> = match record.prev {
            Some(hash_string) => Some(hash_string.parse().map_err(|_| ())?),
            None => None,
        };
        let version = record.version;
        let timestamp = record.time.ok_or(())?.try_into().map_err(|_| ())?;
        let entries: Result<Vec<model::Entry>, ()> = record
            .entries
            .into_iter()
            .map(|proto_entry| proto_entry.try_into())
            .collect();
        let entries = entries?;

        Ok(model::Record {
            prev,
            version,
            timestamp,
            entries,
        })
    }
}

impl TryFrom<protobuf::PackageEntry> for model::Entry {
    type Error = ();

    fn try_from(entry: protobuf::PackageEntry) -> Result<Self, Self::Error> {
        let output = match entry.contents.ok_or(())? {
            protobuf::package_entry::Contents::Init(init) => model::Entry::Init {
                hash_algorithm: init.hash_algorithm.parse().map_err(|_| ())?,
                key: init.key.parse().map_err(|_| ())?,
            },
            protobuf::package_entry::Contents::GrantFlat(grant_flat) => model::Entry::GrantFlat {
                key: grant_flat.key.parse().map_err(|_| ())?,
                permission: grant_flat.permission.try_into()?,
            },
            protobuf::package_entry::Contents::RevokeFlat(revoke_flat) => {
                model::Entry::RevokeFlat {
                    key_id: revoke_flat.key_id.parse().map_err(|_| ())?,
                    permission: revoke_flat.permission.try_into()?,
                }
            }
            protobuf::package_entry::Contents::Release(release) => model::Entry::Release {
                version: release.version.parse().map_err(|_| ())?,
                content: release.content_hash.parse().map_err(|_| ())?,
            },
            protobuf::package_entry::Contents::Yank(yank) => model::Entry::Yank {
                version: yank.version.parse().map_err(|_| ()).map_err(|_| ())?,
            },
        };
        Ok(output)
    }
}

impl TryFrom<i32> for model::Permission {
    type Error = ();

    fn try_from(permission: i32) -> Result<Self, Self::Error> {
        let proto_perm = protobuf::Permission::from_i32(permission).ok_or(())?;
        match proto_perm {
            protobuf::Permission::Release => Ok(model::Permission::Release),
            protobuf::Permission::Yank => Ok(model::Permission::Yank),
        }
    }
}

// Serialization

impl From<model::Record> for Vec<u8> {
    fn from(record: model::Record) -> Self {
        let proto_record: protobuf::PackageRecord = record.into();
        proto_record.encode_to_vec()
    }
}

impl From<model::Record> for protobuf::PackageRecord {
    fn from(record: model::Record) -> Self {
        protobuf::PackageRecord {
            prev: record.prev.map(|hash| hash.to_string()),
            version: record.version,
            time: Some(record.timestamp.into()),
            entries: record
                .entries
                .into_iter()
                .map(|entry| entry.into())
                .collect(),
        }
    }
}

impl From<model::Entry> for protobuf::PackageEntry {
    fn from(entry: model::Entry) -> Self {
        let contents = match entry {
            model::Entry::Init {
                hash_algorithm,
                key,
            } => protobuf::package_entry::Contents::Init(protobuf::Init {
                key: key.to_string(),
                hash_algorithm: hash_algorithm.to_string(),
            }),
            model::Entry::GrantFlat { key, permission } => {
                protobuf::package_entry::Contents::GrantFlat(protobuf::GrantFlat {
                    key: key.to_string(),
                    permission: permission.into(),
                })
            }
            model::Entry::RevokeFlat { key_id, permission } => {
                protobuf::package_entry::Contents::RevokeFlat(protobuf::RevokeFlat {
                    key_id: key_id.to_string(),
                    permission: permission.into(),
                })
            }
            model::Entry::Release { version, content } => {
                protobuf::package_entry::Contents::Release(protobuf::Release {
                    version: version.to_string(),
                    content_hash: content.to_string(),
                })
            }
            model::Entry::Yank { version } => {
                protobuf::package_entry::Contents::Yank(protobuf::Yank {
                    version: version.to_string(),
                })
            }
        };
        let contents = Some(contents);
        protobuf::PackageEntry { contents }
    }
}

impl From<model::Permission> for i32 {
    fn from(permission: model::Permission) -> Self {
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

    use crate::hash::Algorithm as HashAlgorithm;
    use crate::signing::tests::generate_p256_pair;
    use crate::version::Version;

    #[test]
    fn test_envelope_roundtrip() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let (bob_pub, _bob_priv) = generate_p256_pair();

        let record = model::Record {
            prev: None,
            version: 0,
            timestamp: SystemTime::now(),
            entries: vec![
                model::Entry::Init {
                    hash_algorithm: HashAlgorithm::SHA256,
                    key: alice_pub,
                },
                model::Entry::GrantFlat {
                    key: bob_pub.clone(),
                    permission: model::Permission::Release,
                },
                model::Entry::RevokeFlat {
                    key_id: bob_pub.digest(),
                    permission: model::Permission::Release,
                },
                model::Entry::Release {
                    version: Version {
                        major: 1,
                        minor: 2,
                        patch: 0,
                    },
                    content: HashAlgorithm::SHA256.digest(&[0, 1, 2, 3]),
                },
            ],
        };

        let first_envelope = match Envelope::signed_contents(alice_priv, record) {
            Ok(value) => value,
            Err(error) => panic!("Failed to sign envelope 1: {:?}", error),
        };

        let bytes = first_envelope.as_bytes();

        let second_envelope: Envelope<model::Record> = match Envelope::from_bytes(bytes) {
            Ok(value) => value,
            Err(error) => panic!("Failed to create envelope 2: {:?}", error),
        };

        assert_eq!(first_envelope, second_envelope);
    }
}
