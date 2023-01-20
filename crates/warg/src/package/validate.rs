use super::{model, PACKAGE_RECORD_VERSION};
use indexmap::{map::Entry, IndexMap, IndexSet};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;
use warg_crypto::hash::{DynHash, HashAlgorithm};
use warg_crypto::{signing, Signable};

use crate::ProtoEnvelope;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("The first entry of the log is not \"init\"")]
    FirstEntryIsNotInit,

    #[error("The initial record is empty and does not \"init\"")]
    InitialRecordDoesNotInit,

    #[error("The Key ID used to sign this envelope is not known to this package log")]
    KeyIDNotRecognized { key_id: signing::KeyID },

    #[error("A second \"init\" entry was found")]
    InitialEntryAfterBeginning,

    #[error("The key with ID {key_id} did not have required permission {needed_permission}")]
    UnauthorizedAction {
        key_id: signing::KeyID,
        needed_permission: model::Permission,
    },

    #[error("Attempted to remove permission {permission} from key {key_id} which did not have it")]
    PermissionNotFoundToRevoke {
        permission: model::Permission,
        key_id: signing::KeyID,
    },

    #[error("An entry attempted to release already released version {version}")]
    ReleaseOfReleased { version: Version },

    #[error("An entry attempted to yank version {version} which had not yet been released")]
    YankOfUnreleased { version: Version },

    #[error("An entry attempted to yank already yanked version {version}")]
    YankOfYanked { version: Version },

    #[error("Unable to verify signature")]
    SignatureError(#[from] signing::SignatureError),

    #[error("Record hash uses {found} algorithm but {expected} was expected")]
    IncorrectHashAlgorithm {
        found: HashAlgorithm,
        expected: HashAlgorithm,
    },

    #[error("Previous record hash does not match")]
    RecordHashDoesNotMatch,

    #[error("The first record contained a previous hash value")]
    PreviousHashOnFirstRecord,

    #[error("Non-initial record contained no previous hash")]
    NoPreviousHashAfterInit,

    #[error("Protocol version {version} not allowed")]
    ProtocolVersionNotAllowed { version: u32 },

    #[error("Record has lower timestamp than previous")]
    TimestampLowerThanPrevious,
}

/// Represents an index of a key known to the validator.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyIndex(usize);

/// Represents the current state of a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ReleaseState {
    /// The release is currently available.
    Released {
        /// The content digest associated with the release.
        content: DynHash,
    },
    /// The release has been yanked.
    Yanked {
        /// The key index that yanked the package.
        by: KeyIndex,
        /// The timestamp of the yank.
        #[serde(with = "crate::timestamp")]
        timestamp: SystemTime,
    },
}

/// Represents information about a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    /// The version of the release.
    pub version: Version,
    /// The key index that released the package.
    pub by: KeyIndex,
    /// The timestamp of the release.
    #[serde(with = "crate::timestamp")]
    pub timestamp: SystemTime,
    /// The current state of the release.
    pub state: ReleaseState,
}

impl Release {
    /// Determines if the release has been yanked.
    pub fn yanked(&self) -> bool {
        matches!(self.state, ReleaseState::Yanked { .. })
    }

    /// Gets the content associated with the release.
    ///
    /// Returns `None` if the release has been yanked.
    pub fn content(&self) -> Option<&DynHash> {
        match &self.state {
            ReleaseState::Released { content } => Some(content),
            ReleaseState::Yanked { .. } => None,
        }
    }
}

/// Information about the current validation root of the package log.
///
/// A root is the last validated record digest and timestamp.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Root {
    /// The digest of the last validated record.
    pub digest: DynHash,
    /// The timestamp of the last validated record.
    #[serde(with = "crate::timestamp")]
    pub timestamp: SystemTime,
}

/// A validator for package records.
#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validator {
    /// The hash algorithm used by the package log.
    /// This is `None` until the first (i.e. init) record is validated.
    #[serde(skip_serializing_if = "Option::is_none")]
    algorithm: Option<HashAlgorithm>,
    /// The current root of the validator.
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<Root>,
    /// The permissions of each key.
    #[serde(skip_serializing_if = "IndexMap::is_empty", default)]
    permissions: IndexMap<KeyIndex, IndexSet<model::Permission>>,
    /// The releases in the package log.
    #[serde(skip_serializing_if = "IndexMap::is_empty", default)]
    releases: IndexMap<Version, Release>,
    /// The keys known to the validator.
    #[serde(skip_serializing_if = "IndexMap::is_empty", default)]
    keys: IndexMap<signing::KeyID, signing::PublicKey>,
}

impl Validator {
    /// Create a new package log validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the current root of the validator.
    ///
    /// Returns `None` if no records have been validated yet.
    pub fn root(&self) -> &Option<Root> {
        &self.root
    }

    /// Validates an individual package record.
    ///
    /// It is expected that `validate` is called in order of the
    /// records in the log.
    pub fn validate(
        &mut self,
        envelope: &ProtoEnvelope<model::PackageRecord>,
    ) -> Result<Vec<DynHash>, ValidationError> {
        let record = envelope.as_ref();

        // Validate previous hash
        self.validate_record_hash(record)?;

        // Validate version
        self.validate_record_version(record)?;

        // Validate timestamp
        self.validate_record_timestamp(record)?;

        // Validate entries
        let contents =
            self.validate_record_entries(envelope.key_id(), record.timestamp, &record.entries)?;

        // At this point the digest algorithm must be set via an init entry
        let algorithm = self
            .algorithm
            .ok_or(ValidationError::InitialRecordDoesNotInit)?;

        // Validate the envelope key id
        let key = self.keys.get(envelope.key_id()).ok_or_else(|| {
            ValidationError::KeyIDNotRecognized {
                key_id: envelope.key_id().clone(),
            }
        })?;

        // Validate the envelope signature
        model::PackageRecord::verify(key, envelope.content_bytes(), envelope.signature())?;

        // Update the validator root
        self.root = Some(Root {
            digest: algorithm.digest(envelope.content_bytes()),
            timestamp: record.timestamp,
        });

        Ok(contents)
    }

    /// Gets the releases known to the validator.
    ///
    /// The releases are returned in package log order.
    ///
    /// Yanked releases are included.
    pub fn releases(&self) -> impl Iterator<Item = &Release> {
        self.releases.values()
    }

    /// Gets the release with the given version.
    ///
    /// Returns `None` if a release with the given version does not exist.
    pub fn release(&self, version: &Version) -> Option<&Release> {
        self.releases.get(version)
    }

    /// Finds the latest release matching the given version requirement.
    ///
    /// Releases that have been yanked are not considered.
    pub fn find_latest_release(&self, req: &VersionReq) -> Option<&Release> {
        self.releases
            .values()
            .filter(|release| !release.yanked() && req.matches(&release.version))
            .max_by(|a, b| a.version.cmp(&b.version))
    }

    /// Gets a key known to the validator.
    ///
    /// Returns `None` if the given index is invalid.
    pub fn key(&self, index: KeyIndex) -> Option<(&signing::KeyID, &signing::PublicKey)> {
        self.keys.get_index(index.0)
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

    fn validate_record_hash(&self, record: &model::PackageRecord) -> Result<(), ValidationError> {
        match (&self.root, &record.prev) {
            (None, Some(_)) => Err(ValidationError::PreviousHashOnFirstRecord),
            (Some(_), None) => Err(ValidationError::NoPreviousHashAfterInit),
            (None, None) => Ok(()),
            (Some(expected), Some(found)) => {
                if found.algorithm() != expected.digest.algorithm() {
                    return Err(ValidationError::IncorrectHashAlgorithm {
                        found: found.algorithm(),
                        expected: expected.digest.algorithm(),
                    });
                }

                if found != &expected.digest {
                    return Err(ValidationError::RecordHashDoesNotMatch);
                }

                Ok(())
            }
        }
    }

    fn validate_record_version(
        &self,
        record: &model::PackageRecord,
    ) -> Result<(), ValidationError> {
        if record.version == PACKAGE_RECORD_VERSION {
            Ok(())
        } else {
            Err(ValidationError::ProtocolVersionNotAllowed {
                version: record.version,
            })
        }
    }

    fn validate_record_timestamp(
        &self,
        record: &model::PackageRecord,
    ) -> Result<(), ValidationError> {
        if let Some(root) = &self.root {
            if record.timestamp < root.timestamp {
                return Err(ValidationError::TimestampLowerThanPrevious);
            }
        }

        Ok(())
    }

    fn validate_record_entries(
        &mut self,
        signer_key_id: &signing::KeyID,
        timestamp: SystemTime,
        entries: &[model::PackageEntry],
    ) -> Result<Vec<DynHash>, ValidationError> {
        let mut contents = Vec::new();
        let mut signer_key_index = None;

        for entry in entries {
            // Process an init entry specially
            if let model::PackageEntry::Init {
                hash_algorithm,
                key,
            } = entry
            {
                self.validate_init_entry(*hash_algorithm, key)?;
                continue;
            }

            // Must have seen an init entry by now
            if !self.initialized() {
                return Err(ValidationError::FirstEntryIsNotInit);
            }

            if signer_key_index.is_none() {
                signer_key_index =
                    Some(KeyIndex(self.keys.get_index_of(signer_key_id).ok_or_else(
                        || ValidationError::KeyIDNotRecognized {
                            key_id: signer_key_id.clone(),
                        },
                    )?));
            }

            if let Some(permission) = entry.required_permission() {
                self.check_key_permission(signer_key_index.unwrap(), permission)?;
            }

            match entry {
                model::PackageEntry::Init { .. } => unreachable!(), // handled above
                model::PackageEntry::GrantFlat { key, permission } => {
                    self.validate_grant_entry(signer_key_index.unwrap(), key, *permission)?
                }
                model::PackageEntry::RevokeFlat { key_id, permission } => {
                    self.validate_revoke_entry(signer_key_index.unwrap(), key_id, *permission)?
                }
                model::PackageEntry::Release { version, content } => {
                    contents.push(content.clone());
                    self.validate_release_entry(
                        signer_key_index.unwrap(),
                        timestamp,
                        version,
                        content,
                    )?
                }
                model::PackageEntry::Yank { version } => {
                    self.validate_yank_entry(signer_key_index.unwrap(), timestamp, version)?
                }
            }
        }

        Ok(contents)
    }

    fn validate_init_entry(
        &mut self,
        algorithm: HashAlgorithm,
        init_key: &signing::PublicKey,
    ) -> Result<(), ValidationError> {
        if self.initialized() {
            return Err(ValidationError::InitialEntryAfterBeginning);
        }

        assert!(self.permissions.is_empty());
        assert!(self.releases.is_empty());
        assert!(self.keys.is_empty());

        self.algorithm = Some(algorithm);
        let (index, _) = self
            .keys
            .insert_full(init_key.fingerprint(), init_key.clone());
        self.permissions
            .insert(KeyIndex(index), IndexSet::from(model::Permission::all()));

        Ok(())
    }

    fn validate_grant_entry(
        &mut self,
        signer_key_index: KeyIndex,
        key: &signing::PublicKey,
        permission: model::Permission,
    ) -> Result<(), ValidationError> {
        // Check that the current key has the permission they're trying to grant
        self.check_key_permission(signer_key_index, permission)?;

        let (index, _) = self.keys.insert_full(key.fingerprint(), key.clone());
        self.permissions
            .entry(KeyIndex(index))
            .or_default()
            .insert(permission);

        Ok(())
    }

    fn validate_revoke_entry(
        &mut self,
        signer_key_index: KeyIndex,
        key_id: &signing::KeyID,
        permission: model::Permission,
    ) -> Result<(), ValidationError> {
        // Check that the current key has the permission they're trying to revoke
        self.check_key_permission(signer_key_index, permission)?;

        if let Some(set) = self
            .keys
            .get_index_of(key_id)
            .and_then(|index| self.permissions.get_mut(&KeyIndex(index)))
        {
            if set.remove(&permission) {
                return Ok(());
            }
        }

        // Permission not found to remove
        Err(ValidationError::PermissionNotFoundToRevoke {
            permission,
            key_id: key_id.clone(),
        })
    }

    fn validate_release_entry(
        &mut self,
        signer_key_index: KeyIndex,
        timestamp: SystemTime,
        version: &Version,
        content: &DynHash,
    ) -> Result<(), ValidationError> {
        match self.releases.entry(version.clone()) {
            Entry::Occupied(e) => {
                return Err(ValidationError::ReleaseOfReleased {
                    version: e.key().clone(),
                })
            }
            Entry::Vacant(e) => {
                let version = e.key().clone();
                e.insert(Release {
                    version,
                    by: signer_key_index,
                    timestamp,
                    state: ReleaseState::Released {
                        content: content.clone(),
                    },
                });
            }
        }

        Ok(())
    }

    fn validate_yank_entry(
        &mut self,
        signer_key_index: KeyIndex,
        timestamp: SystemTime,
        version: &Version,
    ) -> Result<(), ValidationError> {
        match self.releases.get_mut(version) {
            Some(e) => match e.state {
                ReleaseState::Yanked { .. } => Err(ValidationError::YankOfYanked {
                    version: version.clone(),
                }),
                ReleaseState::Released { .. } => {
                    e.state = ReleaseState::Yanked {
                        by: signer_key_index,
                        timestamp,
                    };
                    Ok(())
                }
            },
            None => Err(ValidationError::YankOfUnreleased {
                version: version.clone(),
            }),
        }
    }

    fn check_key_permission(
        &self,
        key_index: KeyIndex,
        permission: model::Permission,
    ) -> Result<(), ValidationError> {
        if let Some(set) = self.permissions.get(&key_index) {
            if set.contains(&permission) {
                return Ok(());
            }
        }

        // Needed permission not found
        Err(ValidationError::UnauthorizedAction {
            key_id: self.keys.get_index(key_index.0).unwrap().0.clone(),
            needed_permission: permission,
        })
    }
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
        let record = model::PackageRecord {
            prev: None,
            version: PACKAGE_RECORD_VERSION,
            timestamp,
            entries: vec![model::PackageEntry::Init {
                hash_algorithm: HashAlgorithm::Sha256,
                key: alice_pub.clone(),
            }],
        };

        let envelope = ProtoEnvelope::signed_contents(&alice_priv, record).unwrap();
        let mut validator = Validator::default();
        validator.validate(&envelope).unwrap();

        assert_eq!(
            validator,
            Validator {
                root: Some(Root {
                    digest: HashAlgorithm::Sha256.digest(envelope.content_bytes()),
                    timestamp,
                }),
                algorithm: Some(HashAlgorithm::Sha256),
                permissions: IndexMap::from([(
                    KeyIndex(0),
                    IndexSet::from([model::Permission::Release, model::Permission::Yank]),
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
        let mut validator = Validator::default();

        // In envelope 0: alice inits and grants bob release
        let timestamp0 = SystemTime::now();
        let record0 = model::PackageRecord {
            prev: None,
            version: PACKAGE_RECORD_VERSION,
            timestamp: timestamp0,
            entries: vec![
                model::PackageEntry::Init {
                    hash_algorithm: hash_algo,
                    key: alice_pub.clone(),
                },
                model::PackageEntry::GrantFlat {
                    key: bob_pub.clone(),
                    permission: model::Permission::Release,
                },
            ],
        };
        let envelope0 = ProtoEnvelope::signed_contents(&alice_priv, record0).unwrap();
        validator.validate(&envelope0).unwrap();

        // In envelope 1: bob releases 1.1.0
        let timestamp1 = timestamp0 + Duration::from_secs(1);
        let content = hash_algo.digest(&[0, 1, 2, 3]);
        let record1 = model::PackageRecord {
            prev: Some(hash_algo.digest(envelope0.content_bytes())),
            version: PACKAGE_RECORD_VERSION,
            timestamp: timestamp1,
            entries: vec![model::PackageEntry::Release {
                version: Version::new(1, 1, 0),
                content: content.clone(),
            }],
        };

        let envelope1 = ProtoEnvelope::signed_contents(&bob_priv, record1).unwrap();
        validator.validate(&envelope1).unwrap();

        // At this point, the validator should consider 1.1.0 released
        assert_eq!(
            validator.find_latest_release(&"~1".parse().unwrap()),
            Some(&Release {
                version: Version::new(1, 1, 0),
                by: KeyIndex(1),
                timestamp: timestamp1,
                state: ReleaseState::Released {
                    content: content.clone()
                }
            })
        );
        assert!(validator
            .find_latest_release(&"~1.2".parse().unwrap())
            .is_none());
        assert_eq!(
            validator.releases().collect::<Vec<_>>(),
            vec![&Release {
                version: Version::new(1, 1, 0),
                by: KeyIndex(1),
                timestamp: timestamp1,
                state: ReleaseState::Released { content }
            }]
        );

        // In envelope 2: alice revokes bobs access and yanks 1.1.0
        let timestamp2 = timestamp1 + Duration::from_secs(1);
        let record2 = model::PackageRecord {
            prev: Some(hash_algo.digest(envelope1.content_bytes())),
            version: PACKAGE_RECORD_VERSION,
            timestamp: timestamp2,
            entries: vec![
                model::PackageEntry::RevokeFlat {
                    key_id: bob_id.clone(),
                    permission: model::Permission::Release,
                },
                model::PackageEntry::Yank {
                    version: Version::new(1, 1, 0),
                },
            ],
        };
        let envelope2 = ProtoEnvelope::signed_contents(&alice_priv, record2).unwrap();
        validator.validate(&envelope2).unwrap();

        // At this point, the validator should consider 1.1.0 yanked
        assert!(validator
            .find_latest_release(&"~1".parse().unwrap())
            .is_none());
        assert_eq!(
            validator.releases().collect::<Vec<_>>(),
            vec![&Release {
                version: Version::new(1, 1, 0),
                by: KeyIndex(1),
                timestamp: timestamp1,
                state: ReleaseState::Yanked {
                    by: KeyIndex(0),
                    timestamp: timestamp2
                }
            }]
        );

        assert_eq!(
            validator,
            Validator {
                algorithm: Some(HashAlgorithm::Sha256),
                root: Some(Root {
                    digest: HashAlgorithm::Sha256.digest(envelope2.content_bytes()),
                    timestamp: timestamp2,
                }),
                permissions: IndexMap::from([
                    (
                        KeyIndex(0),
                        IndexSet::from([model::Permission::Release, model::Permission::Yank]),
                    ),
                    (KeyIndex(1), IndexSet::default()),
                ]),
                releases: IndexMap::from([(
                    Version::new(1, 1, 0),
                    Release {
                        version: Version::new(1, 1, 0),
                        by: KeyIndex(1),
                        timestamp: timestamp1,
                        state: ReleaseState::Yanked {
                            by: KeyIndex(0),
                            timestamp: timestamp2
                        }
                    }
                )]),
                keys: IndexMap::from([(alice_id, alice_pub), (bob_id, bob_pub)]),
            }
        );
    }
}
