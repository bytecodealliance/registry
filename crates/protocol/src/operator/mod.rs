use crate::{protobuf, registry::RecordId};
use anyhow::{Context, Error};
use prost::Message;
use thiserror::Error;
use warg_crypto::{hash::AnyHash, Decode, Encode, Signable};

mod model;
mod validate;

pub use model::{OperatorEntry, OperatorPermission, OperatorRecord};
pub use validate::{OperatorHead, OperatorState, OperatorValidationError};

/// The currently supported operator protocol version.
pub const OPERATOR_RECORD_VERSION: u32 = 0;

impl Decode for OperatorRecord {
    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        protobuf::OperatorRecord::decode(bytes)?.try_into()
    }
}

impl TryFrom<protobuf::OperatorRecord> for OperatorRecord {
    type Error = Error;

    fn try_from(record: protobuf::OperatorRecord) -> Result<Self, Self::Error> {
        let prev: Option<RecordId> = match record.prev {
            Some(hash_string) => {
                let digest: AnyHash = hash_string.parse()?;
                Some(digest.into())
            }
            None => None,
        };
        let version = record.version;
        let pbjson_timestamp = record.time.context(InvalidTimestampError)?;
        let prost_timestamp = protobuf::pbjson_to_prost_timestamp(pbjson_timestamp);
        let timestamp = prost_timestamp.try_into()?;

        let entries: Result<Vec<OperatorEntry>, Error> = record
            .entries
            .into_iter()
            .map(|proto_entry| proto_entry.try_into())
            .collect();
        let entries = entries?;

        Ok(OperatorRecord {
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

impl TryFrom<protobuf::OperatorEntry> for OperatorEntry {
    type Error = Error;

    fn try_from(entry: protobuf::OperatorEntry) -> Result<Self, Self::Error> {
        use protobuf::operator_entry::Contents;
        let output = match entry.contents.ok_or_else(|| Box::new(EmptyContentError))? {
            Contents::Init(init) => OperatorEntry::Init {
                hash_algorithm: init.hash_algorithm.parse()?,
                key: init.key.parse()?,
            },
            Contents::GrantFlat(grant_flat) => OperatorEntry::GrantFlat {
                key: grant_flat.key.parse()?,
                permission: grant_flat.permission.try_into()?,
            },
            Contents::RevokeFlat(revoke_flat) => OperatorEntry::RevokeFlat {
                key_id: revoke_flat.key_id.into(),
                permission: revoke_flat.permission.try_into()?,
            },
        };
        Ok(output)
    }
}

#[derive(Error, Debug)]
#[error("no content in entry")]
struct EmptyContentError;

impl TryFrom<i32> for OperatorPermission {
    type Error = Error;

    fn try_from(permission: i32) -> Result<Self, Self::Error> {
        let proto_perm = protobuf::OperatorPermission::from_i32(permission)
            .ok_or_else(|| Box::new(OperatorPermissionParseError { value: permission }))?;
        match proto_perm {
            protobuf::OperatorPermission::Unspecified => {
                Err(Error::new(OperatorPermissionParseError {
                    value: permission,
                }))
            }
            protobuf::OperatorPermission::Commit => Ok(OperatorPermission::Commit),
        }
    }
}

#[derive(Error, Debug)]
#[error("the value {value} could not be parsed as a permission")]
struct OperatorPermissionParseError {
    value: i32,
}

// Serialization

impl Signable for OperatorRecord {
    const PREFIX: &'static [u8] = b"WARG-OPERATOR-RECORD-SIGNATURE-V0";
}

impl Encode for OperatorRecord {
    fn encode(&self) -> Vec<u8> {
        let proto_record: protobuf::OperatorRecord = self.into();
        proto_record.encode_to_vec()
    }
}

impl<'a> From<&'a OperatorRecord> for protobuf::OperatorRecord {
    fn from(record: &'a OperatorRecord) -> Self {
        protobuf::OperatorRecord {
            prev: record.prev.as_ref().map(|hash| hash.to_string()),
            version: record.version,
            time: Some(protobuf::prost_to_pbjson_timestamp(record.timestamp.into())),
            entries: record.entries.iter().map(|entry| entry.into()).collect(),
        }
    }
}

impl<'a> From<&'a OperatorEntry> for protobuf::OperatorEntry {
    fn from(entry: &'a OperatorEntry) -> Self {
        use protobuf::operator_entry::Contents;
        let contents = match entry {
            OperatorEntry::Init {
                hash_algorithm,
                key,
            } => Contents::Init(protobuf::OperatorInit {
                key: key.to_string(),
                hash_algorithm: hash_algorithm.to_string(),
            }),
            OperatorEntry::GrantFlat { key, permission } => {
                Contents::GrantFlat(protobuf::OperatorGrantFlat {
                    key: key.to_string(),
                    permission: permission.into(),
                })
            }
            OperatorEntry::RevokeFlat { key_id, permission } => {
                Contents::RevokeFlat(protobuf::OperatorRevokeFlat {
                    key_id: key_id.to_string(),
                    permission: permission.into(),
                })
            }
        };
        let contents = Some(contents);
        protobuf::OperatorEntry { contents }
    }
}

impl<'a> From<&'a OperatorPermission> for i32 {
    fn from(permission: &'a OperatorPermission) -> Self {
        let proto_perm = match permission {
            OperatorPermission::Commit => protobuf::OperatorPermission::Commit,
        };
        proto_perm.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::SystemTime;

    use crate::ProtoEnvelope;
    use warg_crypto::hash::HashAlgorithm;
    use warg_crypto::signing::generate_p256_pair;

    #[test]
    fn test_envelope_roundtrip() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let (bob_pub, _bob_priv) = generate_p256_pair();

        let record = OperatorRecord {
            prev: None,
            version: 0,
            timestamp: SystemTime::now(),
            entries: vec![
                OperatorEntry::Init {
                    hash_algorithm: HashAlgorithm::Sha256,
                    key: alice_pub,
                },
                OperatorEntry::GrantFlat {
                    key: bob_pub.clone(),
                    permission: OperatorPermission::Commit,
                },
                OperatorEntry::RevokeFlat {
                    key_id: bob_pub.fingerprint(),
                    permission: OperatorPermission::Commit,
                },
            ],
        };

        let first_envelope =
            ProtoEnvelope::signed_contents(&alice_priv, record).expect("Failed to sign envelope 1");

        let bytes = first_envelope.to_protobuf();

        let second_envelope: ProtoEnvelope<OperatorRecord> =
            match ProtoEnvelope::from_protobuf(bytes) {
                Ok(value) => value,
                Err(error) => panic!("Failed to create envelope 2: {:?}", error),
            };

        assert_eq!(first_envelope, second_envelope);
    }
}
