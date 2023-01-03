use prost::Message;
use thiserror::Error;

use crate::hash;

pub mod model;
pub mod validate;

use crate::{protobuf, Encode, ErrorBox, Signable};

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
        let pbjson_timestamp = record.time.ok_or_else(|| Box::new(InvalidTimestampError))?;
        let prost_timestamp = protobuf::pbjson_to_prost_timestamp(pbjson_timestamp);
        let timestamp = prost_timestamp.try_into()?;

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
        let proto_perm = protobuf::PackagePermission::from_i32(permission)
            .ok_or_else(|| Box::new(PermissionParseError { value: permission }))?;
        match proto_perm {
            protobuf::PackagePermission::Unspecified => {
                Err(Box::new(PermissionParseError { value: permission }))
            }
            protobuf::PackagePermission::Release => Ok(model::Permission::Release),
            protobuf::PackagePermission::Yank => Ok(model::Permission::Yank),
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
            time: Some(protobuf::prost_to_pbjson_timestamp(record.timestamp.into())),
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
            } => Contents::Init(protobuf::PackageInit {
                key: key.to_string(),
                hash_algorithm: hash_algorithm.to_string(),
            }),
            model::PackageEntry::GrantFlat { key, permission } => {
                Contents::GrantFlat(protobuf::PackageGrantFlat {
                    key: key.to_string(),
                    permission: permission.into(),
                })
            }
            model::PackageEntry::RevokeFlat { key_id, permission } => {
                Contents::RevokeFlat(protobuf::PackageRevokeFlat {
                    key_id: key_id.to_string(),
                    permission: permission.into(),
                })
            }
            model::PackageEntry::Release { version, content } => {
                Contents::Release(protobuf::PackageRelease {
                    version: version.to_string(),
                    content_hash: content.to_string(),
                })
            }
            model::PackageEntry::Yank { version } => Contents::Yank(protobuf::PackageYank {
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
            model::Permission::Release => protobuf::PackagePermission::Release,
            model::Permission::Yank => protobuf::PackagePermission::Yank,
        };
        proto_perm.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::SystemTime;

    use semver::Version;

    use crate::hash::HashAlgorithm;
    use crate::signing::tests::generate_p256_pair;
    use crate::Envelope;

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

        let first_envelope = match Envelope::signed_contents(&alice_priv, record) {
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
