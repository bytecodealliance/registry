use prost::Message;
use thiserror::Error;

use crate::hash;

pub mod model;
pub mod validate;

use crate::{protobuf, Encode, ErrorBox, Signable};

// Deserialization

impl TryFrom<&[u8]> for model::OperatorRecord {
    type Error = ErrorBox;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        protobuf::OperatorRecord::decode(bytes)?.try_into()
    }
}

impl TryFrom<protobuf::OperatorRecord> for model::OperatorRecord {
    type Error = ErrorBox;

    fn try_from(record: protobuf::OperatorRecord) -> Result<Self, Self::Error> {
        let prev: Option<hash::Digest> = match record.prev {
            Some(hash_string) => Some(hash_string.parse()?),
            None => None,
        };
        let version = record.version;
        let pbjson_timestamp = record.time.ok_or_else(|| Box::new(InvalidTimestampError))?;
        let prost_timestamp = protobuf::pbjson_to_prost_timestamp(pbjson_timestamp);
        let timestamp = prost_timestamp.try_into()?;

        let entries: Result<Vec<model::OperatorEntry>, ErrorBox> = record
            .entries
            .into_iter()
            .map(|proto_entry| proto_entry.try_into())
            .collect();
        let entries = entries?;

        Ok(model::OperatorRecord {
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

impl TryFrom<protobuf::OperatorEntry> for model::OperatorEntry {
    type Error = ErrorBox;

    fn try_from(entry: protobuf::OperatorEntry) -> Result<Self, Self::Error> {
        use protobuf::operator_entry::Contents;
        let output = match entry.contents.ok_or_else(|| Box::new(EmptyContentError))? {
            Contents::Init(init) => model::OperatorEntry::Init {
                hash_algorithm: init.hash_algorithm.parse()?,
                key: init.key.parse()?,
            },
            Contents::GrantFlat(grant_flat) => model::OperatorEntry::GrantFlat {
                key: grant_flat.key.parse()?,
                permission: grant_flat.permission.try_into()?,
            },
            Contents::RevokeFlat(revoke_flat) => model::OperatorEntry::RevokeFlat {
                key_id: revoke_flat.key_id.into(),
                permission: revoke_flat.permission.try_into()?,
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
        let proto_perm = protobuf::OperatorPermission::from_i32(permission)
            .ok_or_else(|| Box::new(PermissionParseError { value: permission }))?;
        match proto_perm {
            protobuf::OperatorPermission::Unspecified => Err(Box::new(PermissionParseError { value: permission })),
            protobuf::OperatorPermission::Commit => Ok(model::Permission::Commit),
        }
    }
}

#[derive(Error, Debug)]
#[error("The value {value} could not be parsed as a permission")]
struct PermissionParseError {
    value: i32,
}

// Serialization

impl Signable for model::OperatorRecord {
    const PREFIX: &'static [u8] = b"WARG-OPERATOR-RECORD-SIGNATURE-V0:";
}

impl Encode for model::OperatorRecord {
    fn encode(&self) -> Vec<u8> {
        let proto_record: protobuf::OperatorRecord = self.into();
        proto_record.encode_to_vec()
    }
}

impl<'a> From<&'a model::OperatorRecord> for protobuf::OperatorRecord {
    fn from(record: &'a model::OperatorRecord) -> Self {
        protobuf::OperatorRecord {
            prev: record.prev.as_ref().map(|hash| hash.to_string()),
            version: record.version,
            time: Some(protobuf::prost_to_pbjson_timestamp(record.timestamp.into())),
            entries: record.entries.iter().map(|entry| entry.into()).collect(),
        }
    }
}

impl<'a> From<&'a model::OperatorEntry> for protobuf::OperatorEntry {
    fn from(entry: &'a model::OperatorEntry) -> Self {
        use protobuf::operator_entry::Contents;
        let contents = match entry {
            model::OperatorEntry::Init {
                hash_algorithm,
                key,
            } => Contents::Init(protobuf::OperatorInit {
                key: key.to_string(),
                hash_algorithm: hash_algorithm.to_string(),
            }),
            model::OperatorEntry::GrantFlat { key, permission } => {
                Contents::GrantFlat(protobuf::OperatorGrantFlat {
                    key: key.to_string(),
                    permission: permission.into(),
                })
            }
            model::OperatorEntry::RevokeFlat { key_id, permission } => {
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

impl<'a> From<&'a model::Permission> for i32 {
    fn from(permission: &'a model::Permission) -> Self {
        let proto_perm = match permission {
            model::Permission::Commit => protobuf::OperatorPermission::Commit,
        };
        proto_perm.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::SystemTime;

    use crate::hash::HashAlgorithm;
    use crate::signing::tests::generate_p256_pair;
    use crate::Envelope;

    #[test]
    fn test_envelope_roundtrip() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let (bob_pub, _bob_priv) = generate_p256_pair();

        let record = model::OperatorRecord {
            prev: None,
            version: 0,
            timestamp: SystemTime::now(),
            entries: vec![
                model::OperatorEntry::Init {
                    hash_algorithm: HashAlgorithm::Sha256,
                    key: alice_pub,
                },
                model::OperatorEntry::GrantFlat {
                    key: bob_pub.clone(),
                    permission: model::Permission::Commit,
                },
                model::OperatorEntry::RevokeFlat {
                    key_id: bob_pub.fingerprint(),
                    permission: model::Permission::Commit,
                },
            ],
        };

        let first_envelope = match Envelope::signed_contents(&alice_priv, record) {
            Ok(value) => value,
            Err(error) => panic!("Failed to sign envelope 1: {:?}", error),
        };

        let bytes = first_envelope.to_bytes();

        let second_envelope: Envelope<model::OperatorRecord> = match Envelope::from_bytes(bytes) {
            Ok(value) => value,
            Err(error) => panic!("Failed to create envelope 2: {:?}", error),
        };

        assert_eq!(first_envelope, second_envelope);
    }
}
