use super::{model, PACKAGE_RECORD_VERSION};
use crate::registry::RecordId;
use crate::ProtoEnvelope;
use indexmap::{map::Entry, IndexMap, IndexSet};
use model::{PackageEntry, PackagePermission, PackageRecord};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;
use warg_crypto::hash::{AnyHash, HashAlgorithm, Sha256};
use warg_crypto::{signing, Signable};

#[derive(Error, Debug)]
pub enum PackageValidationError {
    #[error("the first entry of the log is not \"init\"")]
    FirstEntryIsNotInit,

    #[error("the initial record is empty and does not \"init\"")]
    InitialRecordDoesNotInit,

    #[error("the Key ID used to sign this envelope is not known to this package log")]
    KeyIDNotRecognized { key_id: signing::KeyID },

    #[error("a second \"init\" entry was found")]
    InitialEntryAfterBeginning,

    #[error("the key with ID {key_id} did not have required permission {needed_permission}")]
    UnauthorizedAction {
        key_id: signing::KeyID,
        needed_permission: PackagePermission,
    },

    #[error("attempted to remove permission {permission} from key {key_id} which did not have it")]
    PermissionNotFoundToRevoke {
        permission: PackagePermission,
        key_id: signing::KeyID,
    },

    #[error("an entry attempted to release version {version} which is already released")]
    ReleaseOfReleased { version: Version },

    #[error("an entry attempted to yank version {version} which had not yet been released")]
    YankOfUnreleased { version: Version },

    #[error("an entry attempted to yank version {version} which is already yanked")]
    YankOfYanked { version: Version },

    #[error("unable to verify signature")]
    SignatureError(#[from] signing::SignatureError),

    #[error("record hash uses {found} algorithm but {expected} was expected")]
    IncorrectHashAlgorithm {
        found: HashAlgorithm,
        expected: HashAlgorithm,
    },

    #[error("previous record hash does not match")]
    RecordHashDoesNotMatch,

    #[error("the first record contained a previous hash value")]
    PreviousHashOnFirstRecord,

    #[error("non-initial record contained no previous hash")]
    NoPreviousHashAfterInit,

    #[error("protocol version {version} not allowed")]
    ProtocolVersionNotAllowed { version: u32 },

    #[error("record has lower timestamp than previous")]
    TimestampLowerThanPrevious,
}

/// Represents the current state of a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PackageReleaseState {
    /// The release is currently available.
    Released {
        /// The content digest associated with the release.
        content: AnyHash,
    },
    /// The release has been yanked.
    Yanked {
        /// The key id that yanked the package.
        by: signing::KeyID,
        /// The timestamp of the yank.
        #[serde(with = "crate::timestamp")]
        timestamp: SystemTime,
    },
}

/// Represents information about a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageRelease {
    /// The id of the record that released the package.
    pub record_id: RecordId,
    /// The version of the release.
    pub version: Version,
    /// The key id that released the package.
    pub by: signing::KeyID,
    /// The timestamp of the release.
    #[serde(with = "crate::timestamp")]
    pub timestamp: SystemTime,
    /// The current state of the release.
    pub state: PackageReleaseState,
}

impl PackageRelease {
    /// Determines if the release has been yanked.
    pub fn yanked(&self) -> bool {
        matches!(self.state, PackageReleaseState::Yanked { .. })
    }

    /// Gets the content associated with the release.
    ///
    /// Returns `None` if the release has been yanked.
    pub fn content(&self) -> Option<&AnyHash> {
        match &self.state {
            PackageReleaseState::Released { content } => Some(content),
            PackageReleaseState::Yanked { .. } => None,
        }
    }
}

/// Information about the current validation head of the package log.
///
/// A head is the last validated record digest and timestamp.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageHead {
    /// The digest of the last validated record.
    pub digest: RecordId,
    /// The timestamp of the last validated record.
    #[serde(with = "crate::timestamp")]
    pub timestamp: SystemTime,
}

/// A validator for package records.
#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct PackageState {
    /// The hash algorithm used by the package log.
    /// This is `None` until the first (i.e. init) record is validated.
    #[serde(skip_serializing_if = "Option::is_none")]
    algorithm: Option<HashAlgorithm>,
    /// The current head of the validator.
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<PackageHead>,
    /// The permissions of each key.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    permissions: IndexMap<signing::KeyID, IndexSet<PackagePermission>>,
    /// The releases in the package log.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    releases: IndexMap<Version, PackageRelease>,
    /// The keys known thus far processing the package records.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    keys: IndexMap<signing::KeyID, signing::PublicKey>,
}

impl PackageState {
    /// Create a new package log validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the current head of the validator.
    ///
    /// Returns `None` if no records have been validated yet.
    pub fn head(&self) -> &Option<PackageHead> {
        &self.head
    }

    /// Validates an individual package record.
    ///
    /// It is expected that `validate` is called in order of the
    /// records in the log.
    ///
    /// This operation is transactional: if any entry in the record
    /// fails to validate, the validator state will remain unchanged.
    pub fn validate(
        &mut self,
        record: &ProtoEnvelope<PackageRecord>,
    ) -> Result<(), PackageValidationError> {
        let snapshot = self.snapshot();

        let result = self.validate_record(record);
        if result.is_err() {
            self.rollback(snapshot);
        }

        result
    }

    /// Gets the releases known to the validator.
    ///
    /// The releases are returned in package log order.
    ///
    /// Yanked releases are included.
    pub fn releases(&self) -> impl Iterator<Item = &PackageRelease> {
        self.releases.values()
    }

    /// Gets the release with the given version.
    ///
    /// Returns `None` if a release with the given version does not exist.
    pub fn release(&self, version: &Version) -> Option<&PackageRelease> {
        self.releases.get(version)
    }

    /// Finds the latest release matching the given version requirement.
    ///
    /// Releases that have been yanked are not considered.
    pub fn find_latest_release(&self, req: &VersionReq) -> Option<&PackageRelease> {
        self.releases
            .values()
            .filter(|release| !release.yanked() && req.matches(&release.version))
            .max_by(|a, b| a.version.cmp(&b.version))
    }

    /// Gets the public key of the given key id.
    ///
    /// Returns `None` if the key id is not recognized.
    pub fn public_key(&self, key_id: &signing::KeyID) -> Option<&signing::PublicKey> {
        self.keys.get(key_id)
    }

    fn initialized(&self) -> bool {
        // The package log is initialized if the hash algorithm is set
        self.algorithm.is_some()
    }

    fn validate_record(
        &mut self,
        envelope: &ProtoEnvelope<PackageRecord>,
    ) -> Result<(), PackageValidationError> {
        let record = envelope.as_ref();
        let record_id = RecordId::package_record::<Sha256>(envelope);

        // Validate previous hash
        self.validate_record_hash(record)?;

        // Validate version
        self.validate_record_version(record)?;

        // Validate timestamp
        self.validate_record_timestamp(record)?;

        // Validate entries
        self.validate_record_entries(
            &record_id,
            envelope.key_id(),
            record.timestamp,
            &record.entries,
        )?;

        // At this point the digest algorithm must be set via an init entry
        let _algorithm = self
            .algorithm
            .ok_or(PackageValidationError::InitialRecordDoesNotInit)?;

        // Validate the envelope key id
        let key = self.keys.get(envelope.key_id()).ok_or_else(|| {
            PackageValidationError::KeyIDNotRecognized {
                key_id: envelope.key_id().clone(),
            }
        })?;

        // Validate the envelope signature
        PackageRecord::verify(key, envelope.content_bytes(), envelope.signature())?;

        // Update the validator head
        self.head = Some(PackageHead {
            digest: record_id,
            timestamp: record.timestamp,
        });

        Ok(())
    }

    fn validate_record_hash(&self, record: &PackageRecord) -> Result<(), PackageValidationError> {
        match (&self.head, &record.prev) {
            (None, Some(_)) => Err(PackageValidationError::PreviousHashOnFirstRecord),
            (Some(_), None) => Err(PackageValidationError::NoPreviousHashAfterInit),
            (None, None) => Ok(()),
            (Some(expected), Some(found)) => {
                if found.algorithm() != expected.digest.algorithm() {
                    return Err(PackageValidationError::IncorrectHashAlgorithm {
                        found: found.algorithm(),
                        expected: expected.digest.algorithm(),
                    });
                }

                if found != &expected.digest {
                    return Err(PackageValidationError::RecordHashDoesNotMatch);
                }

                Ok(())
            }
        }
    }

    fn validate_record_version(
        &self,
        record: &PackageRecord,
    ) -> Result<(), PackageValidationError> {
        if record.version == PACKAGE_RECORD_VERSION {
            Ok(())
        } else {
            Err(PackageValidationError::ProtocolVersionNotAllowed {
                version: record.version,
            })
        }
    }

    fn validate_record_timestamp(
        &self,
        record: &PackageRecord,
    ) -> Result<(), PackageValidationError> {
        if let Some(head) = &self.head {
            if record.timestamp < head.timestamp {
                return Err(PackageValidationError::TimestampLowerThanPrevious);
            }
        }

        Ok(())
    }

    fn validate_record_entries(
        &mut self,
        record_id: &RecordId,
        signer_key_id: &signing::KeyID,
        timestamp: SystemTime,
        entries: &[PackageEntry],
    ) -> Result<(), PackageValidationError> {
        for entry in entries {
            if let Some(permission) = entry.required_permission() {
                self.check_key_permission(signer_key_id, permission)?;
            }

            // Process an init entry specially
            if let PackageEntry::Init {
                hash_algorithm,
                key,
            } = entry
            {
                self.validate_init_entry(signer_key_id, *hash_algorithm, key)?;
                continue;
            }

            // Must have seen an init entry by now
            if !self.initialized() {
                return Err(PackageValidationError::FirstEntryIsNotInit);
            }

            match entry {
                PackageEntry::Init { .. } => unreachable!(), // handled above
                PackageEntry::GrantFlat { key, permission } => {
                    self.validate_grant_entry(signer_key_id, key, *permission)?
                }
                PackageEntry::RevokeFlat { key_id, permission } => {
                    self.validate_revoke_entry(signer_key_id, key_id, *permission)?
                }
                PackageEntry::Release { version, content } => self.validate_release_entry(
                    record_id,
                    signer_key_id,
                    timestamp,
                    version,
                    content,
                )?,
                PackageEntry::Yank { version } => {
                    self.validate_yank_entry(signer_key_id, timestamp, version)?
                }
            }
        }

        Ok(())
    }

    fn validate_init_entry(
        &mut self,
        signer_key_id: &signing::KeyID,
        algorithm: HashAlgorithm,
        init_key: &signing::PublicKey,
    ) -> Result<(), PackageValidationError> {
        if self.initialized() {
            return Err(PackageValidationError::InitialEntryAfterBeginning);
        }

        assert!(self.permissions.is_empty());
        assert!(self.releases.is_empty());
        assert!(self.keys.is_empty());

        self.algorithm = Some(algorithm);
        self.permissions.insert(
            signer_key_id.clone(),
            IndexSet::from(PackagePermission::all()),
        );
        self.keys.insert(init_key.fingerprint(), init_key.clone());

        Ok(())
    }

    fn validate_grant_entry(
        &mut self,
        signer_key_id: &signing::KeyID,
        key: &signing::PublicKey,
        permission: PackagePermission,
    ) -> Result<(), PackageValidationError> {
        // Check that the current key has the permission they're trying to grant
        self.check_key_permission(signer_key_id, permission)?;

        let grant_key_id = key.fingerprint();
        self.keys.insert(grant_key_id.clone(), key.clone());
        self.permissions
            .entry(grant_key_id)
            .or_default()
            .insert(permission);

        Ok(())
    }

    fn validate_revoke_entry(
        &mut self,
        signer_key_id: &signing::KeyID,
        key_id: &signing::KeyID,
        permission: PackagePermission,
    ) -> Result<(), PackageValidationError> {
        // Check that the current key has the permission they're trying to revoke
        self.check_key_permission(signer_key_id, permission)?;

        if let Some(set) = self.permissions.get_mut(key_id) {
            if set.remove(&permission) {
                return Ok(());
            }
        }

        // Permission not found to remove
        Err(PackageValidationError::PermissionNotFoundToRevoke {
            permission,
            key_id: key_id.clone(),
        })
    }

    fn validate_release_entry(
        &mut self,
        record_id: &RecordId,
        signer_key_id: &signing::KeyID,
        timestamp: SystemTime,
        version: &Version,
        content: &AnyHash,
    ) -> Result<(), PackageValidationError> {
        match self.releases.entry(version.clone()) {
            Entry::Occupied(e) => {
                return Err(PackageValidationError::ReleaseOfReleased {
                    version: e.key().clone(),
                })
            }
            Entry::Vacant(e) => {
                let version = e.key().clone();
                e.insert(PackageRelease {
                    record_id: record_id.clone(),
                    version,
                    by: signer_key_id.clone(),
                    timestamp,
                    state: PackageReleaseState::Released {
                        content: content.clone(),
                    },
                });
            }
        }

        Ok(())
    }

    fn validate_yank_entry(
        &mut self,
        signer_key_id: &signing::KeyID,
        timestamp: SystemTime,
        version: &Version,
    ) -> Result<(), PackageValidationError> {
        match self.releases.get_mut(version) {
            Some(e) => match e.state {
                PackageReleaseState::Yanked { .. } => Err(PackageValidationError::YankOfYanked {
                    version: version.clone(),
                }),
                PackageReleaseState::Released { .. } => {
                    e.state = PackageReleaseState::Yanked {
                        by: signer_key_id.clone(),
                        timestamp,
                    };
                    Ok(())
                }
            },
            None => Err(PackageValidationError::YankOfUnreleased {
                version: version.clone(),
            }),
        }
    }

    fn check_key_permission(
        &self,
        key_id: &signing::KeyID,
        permission: PackagePermission,
    ) -> Result<(), PackageValidationError> {
        if let Some(available_permissions) = self.permissions.get(key_id) {
            if available_permissions.contains(&permission) {
                return Ok(());
            }
        }

        // Needed permission not found
        Err(PackageValidationError::UnauthorizedAction {
            key_id: key_id.clone(),
            needed_permission: permission,
        })
    }

    fn snapshot(&self) -> Snapshot {
        let Self {
            algorithm,
            head,
            releases,
            permissions,
            keys,
        } = self;

        Snapshot {
            algorithm: *algorithm,
            head: head.clone(),
            releases: releases.len(),
            permissions: permissions.len(),
            keys: keys.len(),
        }
    }

    fn rollback(&mut self, snapshot: Snapshot) {
        let Snapshot {
            algorithm,
            head,
            releases,
            permissions,
            keys,
        } = snapshot;

        self.algorithm = algorithm;
        self.head = head;
        self.releases.truncate(releases);
        self.permissions.truncate(permissions);
        self.keys.truncate(keys);
    }
}

impl crate::Validator for PackageState {
    type Record = PackageRecord;
    type Error = PackageValidationError;

    fn validate(&mut self, record: &ProtoEnvelope<Self::Record>) -> Result<(), Self::Error> {
        self.validate(record)
    }
}

struct Snapshot {
    algorithm: Option<HashAlgorithm>,
    head: Option<PackageHead>,
    releases: usize,
    permissions: usize,
    keys: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::time::{Duration, SystemTime};
    use warg_crypto::hash::HashAlgorithm;
    use warg_crypto::signing::generate_p256_pair;

    #[test]
    fn test_validate_base_log() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let alice_id = alice_pub.fingerprint();

        let timestamp = SystemTime::now();
        let record = PackageRecord {
            prev: None,
            version: PACKAGE_RECORD_VERSION,
            timestamp,
            entries: vec![PackageEntry::Init {
                hash_algorithm: HashAlgorithm::Sha256,
                key: alice_pub.clone(),
            }],
        };

        let envelope = ProtoEnvelope::signed_contents(&alice_priv, record).unwrap();
        let mut package_state = PackageState::default();
        package_state.validate(&envelope).unwrap();

        assert_eq!(
            package_state,
            PackageState {
                head: Some(PackageHead {
                    digest: RecordId::package_record::<Sha256>(&envelope),
                    timestamp,
                }),
                algorithm: Some(HashAlgorithm::Sha256),
                permissions: IndexMap::from([(
                    alice_id.clone(),
                    IndexSet::from([PackagePermission::Release, PackagePermission::Yank]),
                )]),
                releases: IndexMap::default(),
                keys: IndexMap::from([(alice_id, alice_pub)]),
            }
        );
    }

    #[test]
    fn test_validate_larger_log() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let (bob_pub, bob_priv) = generate_p256_pair();
        let alice_id = alice_pub.fingerprint();
        let bob_id = bob_pub.fingerprint();

        let hash_algo = HashAlgorithm::Sha256;
        let mut package_state = PackageState::default();

        // In envelope 0: alice inits and grants bob release
        let timestamp0 = SystemTime::now();
        let record0 = PackageRecord {
            prev: None,
            version: PACKAGE_RECORD_VERSION,
            timestamp: timestamp0,
            entries: vec![
                PackageEntry::Init {
                    hash_algorithm: hash_algo,
                    key: alice_pub.clone(),
                },
                PackageEntry::GrantFlat {
                    key: bob_pub.clone(),
                    permission: PackagePermission::Release,
                },
            ],
        };
        let envelope0 = ProtoEnvelope::signed_contents(&alice_priv, record0).unwrap();
        package_state.validate(&envelope0).unwrap();

        // In envelope 1: bob releases 1.1.0
        let timestamp1 = timestamp0 + Duration::from_secs(1);
        let content = hash_algo.digest(&[0, 1, 2, 3]);
        let record1 = PackageRecord {
            prev: Some(RecordId::package_record::<Sha256>(&envelope0)),
            version: PACKAGE_RECORD_VERSION,
            timestamp: timestamp1,
            entries: vec![PackageEntry::Release {
                version: Version::new(1, 1, 0),
                content: content.clone(),
            }],
        };

        let envelope1 = ProtoEnvelope::signed_contents(&bob_priv, record1).unwrap();
        let record_id1 = RecordId::package_record::<Sha256>(&envelope1);
        package_state.validate(&envelope1).unwrap();

        // At this point, the package_state should consider 1.1.0 released
        assert_eq!(
            package_state.find_latest_release(&"~1".parse().unwrap()),
            Some(&PackageRelease {
                record_id: record_id1.clone(),
                version: Version::new(1, 1, 0),
                by: bob_id.clone(),
                timestamp: timestamp1,
                state: PackageReleaseState::Released {
                    content: content.clone()
                }
            })
        );
        assert!(package_state
            .find_latest_release(&"~1.2".parse().unwrap())
            .is_none());
        assert_eq!(
            package_state.releases().collect::<Vec<_>>(),
            vec![&PackageRelease {
                record_id: record_id1.clone(),
                version: Version::new(1, 1, 0),
                by: bob_id.clone(),
                timestamp: timestamp1,
                state: PackageReleaseState::Released { content }
            }]
        );

        // In envelope 2: alice revokes bobs access and yanks 1.1.0
        let timestamp2 = timestamp1 + Duration::from_secs(1);
        let record2 = PackageRecord {
            prev: Some(RecordId::package_record::<Sha256>(&envelope1)),
            version: PACKAGE_RECORD_VERSION,
            timestamp: timestamp2,
            entries: vec![
                PackageEntry::RevokeFlat {
                    key_id: bob_id.clone(),
                    permission: PackagePermission::Release,
                },
                PackageEntry::Yank {
                    version: Version::new(1, 1, 0),
                },
            ],
        };
        let envelope2 = ProtoEnvelope::signed_contents(&alice_priv, record2).unwrap();
        package_state.validate(&envelope2).unwrap();

        // At this point, the package_state should consider 1.1.0 yanked
        assert!(package_state
            .find_latest_release(&"~1".parse().unwrap())
            .is_none());
        assert_eq!(
            package_state.releases().collect::<Vec<_>>(),
            vec![&PackageRelease {
                record_id: record_id1.clone(),
                version: Version::new(1, 1, 0),
                by: bob_id.clone(),
                timestamp: timestamp1,
                state: PackageReleaseState::Yanked {
                    by: alice_id.clone(),
                    timestamp: timestamp2
                }
            }]
        );

        assert_eq!(
            package_state,
            PackageState {
                algorithm: Some(HashAlgorithm::Sha256),
                head: Some(PackageHead {
                    digest: RecordId::package_record::<Sha256>(&envelope2),
                    timestamp: timestamp2,
                }),
                permissions: IndexMap::from([
                    (
                        alice_id.clone(),
                        IndexSet::from([PackagePermission::Release, PackagePermission::Yank]),
                    ),
                    (bob_id.clone(), IndexSet::default()),
                ]),
                releases: IndexMap::from([(
                    Version::new(1, 1, 0),
                    PackageRelease {
                        record_id: record_id1,
                        version: Version::new(1, 1, 0),
                        by: bob_id.clone(),
                        timestamp: timestamp1,
                        state: PackageReleaseState::Yanked {
                            by: alice_id.clone(),
                            timestamp: timestamp2
                        }
                    }
                )]),
                keys: IndexMap::from([(alice_id, alice_pub), (bob_id, bob_pub),]),
            }
        );
    }

    #[test]
    fn test_rollback() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let alice_id = alice_pub.fingerprint();
        let (bob_pub, _) = generate_p256_pair();

        let timestamp = SystemTime::now();
        let record = PackageRecord {
            prev: None,
            version: 0,
            timestamp,
            entries: vec![PackageEntry::Init {
                hash_algorithm: HashAlgorithm::Sha256,
                key: alice_pub.clone(),
            }],
        };

        let envelope =
            ProtoEnvelope::signed_contents(&alice_priv, record).expect("failed to sign envelope");
        let mut package_state = PackageState::default();
        package_state.validate(&envelope).unwrap();

        let expected = PackageState {
            head: Some(PackageHead {
                digest: RecordId::package_record::<Sha256>(&envelope),
                timestamp,
            }),
            algorithm: Some(HashAlgorithm::Sha256),
            releases: IndexMap::new(),
            permissions: IndexMap::from([(
                alice_id.clone(),
                IndexSet::from([PackagePermission::Release, PackagePermission::Yank]),
            )]),
            keys: IndexMap::from([(alice_id, alice_pub)]),
        };

        assert_eq!(package_state, expected);

        let record = PackageRecord {
            prev: Some(RecordId::package_record::<Sha256>(&envelope)),
            version: 0,
            timestamp: SystemTime::now(),
            entries: vec![
                // This entry is valid
                PackageEntry::GrantFlat {
                    key: bob_pub,
                    permission: PackagePermission::Release,
                },
                // This entry is not valid
                PackageEntry::RevokeFlat {
                    key_id: "not-valid".to_string().into(),
                    permission: PackagePermission::Release,
                },
            ],
        };

        let envelope =
            ProtoEnvelope::signed_contents(&alice_priv, record).expect("failed to sign envelope");

        // This validation should fail and the package_state should remain unchanged
        match package_state.validate(&envelope).unwrap_err() {
            PackageValidationError::PermissionNotFoundToRevoke { .. } => {}
            _ => panic!("expected a different error"),
        }

        // The package_state should not have changed
        assert_eq!(package_state, expected);
    }
}
