use std::time::SystemTime;

use hashbrown::{HashMap, HashSet};
use semver::Version;
use signature::Error as SignatureError;
use thiserror::Error;

use crate::hash;
use crate::signing;

use super::model;
use super::Envelope;
use super::Signable;

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
    SignatureError(#[from] SignatureError),

    #[error("Record hash uses {found} algorithm but {expected} was expected")]
    IncorrectHashAlgorithm {
        found: hash::HashAlgorithm,
        expected: hash::HashAlgorithm,
    },

    #[error("Previous record hash does not match")]
    RecordHashDoesntMatch,

    #[error("The first record contained a previous hash value")]
    PreviousHashOnFirstRecord,

    #[error("Non-initial record contained no previous hash")]
    NoPreviousHashAfterInit,

    #[error("Protocol version {version} not allowed")]
    ProtocolVersionNotAllowed { version: u32 },

    #[error("Record has lower timestamp than previous")]
    TimestampLowerThanPrevious,
}

#[allow(clippy::large_enum_variant)]
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum ValidationState {
    #[default]
    Uninitialized,
    Initialized(ValidationStateInit),
}

/// The state known to record validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationStateInit {
    /// Last record hash
    last_record: hash::Digest,
    /// Last timestamp
    last_timestamp: SystemTime,
    /// The state known to entry validation
    entry_state: EntryValidationState,
}

impl ValidationState {
    /// Determine the state of the validator (or error) after the next
    /// package record envelope has been processed.
    pub fn validate_envelope(
        self,
        envelope: &Envelope<model::PackageRecord>,
    ) -> Result<ValidationState, ValidationError> {
        let state = self.validate_record(envelope.key_id.clone(), envelope)?;

        if let ValidationState::Initialized(initialized_state) = &state {
            if let Some(key) = initialized_state
                .entry_state
                .known_keys
                .get(&envelope.key_id)
            {
                model::PackageRecord::verify(
                    key.clone(),
                    &envelope.content_bytes,
                    &envelope.signature,
                )?;
            } else {
                return Err(ValidationError::KeyIDNotRecognized {
                    key_id: envelope.key_id.clone(),
                });
            }
        } else {
            return Err(ValidationError::InitialRecordDoesNotInit);
        }

        Ok(state)
    }

    pub fn validate_record(
        self,
        key_id: signing::KeyID,
        envelope: &Envelope<model::PackageRecord>,
    ) -> Result<ValidationState, ValidationError> {
        let record = &envelope.contents;

        // Validate previous hash
        self.validate_record_hash(record)?;

        // Validate version
        self.validate_record_version(record)?;

        // Validate timestamp
        self.validate_record_timestamp(record)?;

        // Validate entries
        let entry_state = self.validate_record_entries(key_id, record)?;

        let last_record = entry_state.hash_algorithm.digest(&envelope.content_bytes);
        let last_timestamp = record.timestamp;
        Ok(ValidationState::Initialized(ValidationStateInit {
            last_record,
            last_timestamp,
            entry_state,
        }))
    }

    fn validate_record_hash(&self, record: &model::PackageRecord) -> Result<(), ValidationError> {
        match (&self, &record.prev) {
            (ValidationState::Uninitialized, None) => Ok(()),
            (ValidationState::Uninitialized, Some(_)) => {
                Err(ValidationError::PreviousHashOnFirstRecord)
            }
            (ValidationState::Initialized(_), None) => {
                Err(ValidationError::NoPreviousHashAfterInit)
            }
            (ValidationState::Initialized(state), Some(prev)) => {
                if prev.algorithm() != state.entry_state.hash_algorithm {
                    return Err(ValidationError::IncorrectHashAlgorithm {
                        found: prev.algorithm(),
                        expected: state.entry_state.hash_algorithm,
                    });
                }
                if prev != &state.last_record {
                    return Err(ValidationError::RecordHashDoesntMatch);
                }
                Ok(())
            }
        }
    }

    fn validate_record_version(
        &self,
        record: &model::PackageRecord,
    ) -> Result<(), ValidationError> {
        if record.version == 0 {
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
        if let ValidationState::Initialized(state) = &self {
            if record.timestamp < state.last_timestamp {
                return Err(ValidationError::TimestampLowerThanPrevious);
            }
        }
        Ok(())
    }

    fn validate_record_entries(
        self,
        key_id: signing::KeyID,
        record: &model::PackageRecord,
    ) -> Result<EntryValidationState, ValidationError> {
        let mut entry_validation_state = match self {
            ValidationState::Uninitialized => None,
            ValidationState::Initialized(state) => Some(state.entry_state),
        };

        for entry in &record.entries {
            entry_validation_state = match entry_validation_state {
                Some(state) => Some(state.validate_next(key_id.clone(), entry)?),
                None => Some(EntryValidationState::validate_first(key_id.clone(), entry)?),
            };
        }

        match entry_validation_state {
            Some(state) => Ok(state),
            None => Err(ValidationError::InitialRecordDoesNotInit),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryValidationState {
    /// The hash algorithm used by this package
    hash_algorithm: hash::HashAlgorithm,
    /// The permissions associated with a given key_id
    permissions: HashMap<signing::KeyID, HashSet<model::Permission>>,
    /// The state of all releases
    releases: HashMap<Version, ReleaseState>,
    /// The relevant known keys to this
    known_keys: HashMap<signing::KeyID, signing::PublicKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseState {
    Unreleased,
    Released { content: hash::Digest },
    Yanked,
}

impl EntryValidationState {
    pub fn validate_first(
        key_id: signing::KeyID,
        entry: &model::PackageEntry,
    ) -> Result<Self, ValidationError> {
        match entry {
            model::PackageEntry::Init {
                hash_algorithm,
                key: init_key,
            } => Ok(EntryValidationState {
                hash_algorithm: *hash_algorithm,
                permissions: HashMap::from([(
                    key_id,
                    HashSet::from([model::Permission::Release, model::Permission::Yank]),
                )]),
                releases: HashMap::default(),
                known_keys: HashMap::from([(init_key.fingerprint(), init_key.clone())]),
            }),
            _ => Err(ValidationError::FirstEntryIsNotInit),
        }
    }

    pub fn validate_next(
        mut self,
        key_id: signing::KeyID,
        entry: &model::PackageEntry,
    ) -> Result<EntryValidationState, ValidationError> {
        if let Some(needed_permission) = entry.required_permission() {
            self.check_key_permission(key_id, needed_permission)?;
        }

        match entry {
            // Invalid re-initialization
            model::PackageEntry::Init { .. } => Err(ValidationError::InitialEntryAfterBeginning),

            model::PackageEntry::GrantFlat { key, permission } => {
                let grant_key_id = key.fingerprint();
                self.known_keys.insert(grant_key_id.clone(), key.clone());

                match self.permissions.entry(grant_key_id) {
                    hashbrown::hash_map::Entry::Occupied(mut entry) => {
                        entry.get_mut().insert(*permission);
                    }
                    hashbrown::hash_map::Entry::Vacant(entry) => {
                        entry.insert(HashSet::from([*permission]));
                    }
                };

                Ok(self)
            }

            model::PackageEntry::RevokeFlat { key_id, permission } => {
                match self.permissions.entry(key_id.clone()) {
                    hashbrown::hash_map::Entry::Occupied(mut entry) => {
                        let permissions_set = entry.get_mut();
                        if !permissions_set.contains(permission) {
                            return Err(ValidationError::PermissionNotFoundToRevoke {
                                permission: *permission,
                                key_id: key_id.clone(),
                            });
                        }
                        entry.get_mut().remove(permission);
                    }
                    hashbrown::hash_map::Entry::Vacant(_) => {
                        return Err(ValidationError::PermissionNotFoundToRevoke {
                            permission: *permission,
                            key_id: key_id.clone(),
                        })
                    }
                };
                Ok(self)
            }

            model::PackageEntry::Release { version, content } => {
                let version = version.clone();
                let content = content.clone();

                // Check the state of the specified version
                let old_state = self
                    .releases
                    .get(&version)
                    .cloned()
                    .unwrap_or(ReleaseState::Unreleased);

                match old_state {
                    ReleaseState::Unreleased => {
                        self.releases
                            .insert(version, ReleaseState::Released { content });
                        Ok(self)
                    }
                    ReleaseState::Released { content: _ } => {
                        Err(ValidationError::ReleaseOfReleased { version })
                    }
                    ReleaseState::Yanked => Err(ValidationError::ReleaseOfReleased { version }),
                }
            }

            model::PackageEntry::Yank { version } => {
                let version = version.clone();

                // Check the state of the specified version
                let old_state = self
                    .releases
                    .get(&version)
                    .cloned()
                    .unwrap_or(ReleaseState::Unreleased);

                match old_state {
                    ReleaseState::Unreleased => Err(ValidationError::YankOfUnreleased { version }),
                    ReleaseState::Released { content: _ } => {
                        self.releases.insert(version, ReleaseState::Yanked);
                        Ok(self)
                    }
                    ReleaseState::Yanked => Err(ValidationError::YankOfYanked { version }),
                }
            }
        }
    }

    fn check_key_permission(
        &self,
        key_id: signing::KeyID,
        permission: model::Permission,
    ) -> Result<(), ValidationError> {
        if let Some(available_permissions) = self.permissions.get(&key_id) {
            if available_permissions.contains(&permission) {
                return Ok(()); // Needed permission found
            }
        }

        // Needed permission not found
        Err(ValidationError::UnauthorizedAction {
            key_id,
            needed_permission: permission,
        })
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::signing::tests::generate_p256_pair;

    use crate::hash::HashAlgorithm;
    use std::time::SystemTime;

    #[test]
    fn test_validate_base_log() {
        let (alice_pub, alice_priv) = generate_p256_pair();

        let timestamp = SystemTime::now();
        let record = model::PackageRecord {
            prev: None,
            version: 0,
            timestamp,
            entries: vec![model::PackageEntry::Init {
                hash_algorithm: HashAlgorithm::Sha256,
                key: alice_pub.clone(),
            }],
        };

        let envelope = match Envelope::signed_contents(alice_priv, record) {
            Ok(value) => value,
            Err(error) => panic!("Failed to sign envelope: {:?}", error),
        };

        let validation_state = match ValidationState::Uninitialized.validate_envelope(&envelope) {
            Ok(value) => value,
            Err(error) => panic!("Failed to validate: {:?}", error),
        };

        let expected_state = ValidationState::Initialized(ValidationStateInit {
            last_record: HashAlgorithm::Sha256.digest(&envelope.content_bytes),
            last_timestamp: timestamp,
            entry_state: EntryValidationState {
                hash_algorithm: HashAlgorithm::Sha256,
                permissions: HashMap::from([(
                    alice_pub.fingerprint(),
                    HashSet::from([model::Permission::Release, model::Permission::Yank]),
                )]),
                releases: HashMap::default(),
                known_keys: HashMap::from([(alice_pub.fingerprint(), alice_pub.clone())]),
            },
        });

        assert_eq!(expected_state, validation_state);
    }
}
