use super::model;
use crate::{hash, signing, Envelope, Signable};
use semver::Version;
use signature::Error as SignatureError;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    time::SystemTime,
};
use thiserror::Error;

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
    pub entry_state: EntryValidationState,
}

impl ValidationState {
    /// Determine the state of the validator (or error) after the next
    /// package record envelope has been processed.
    pub fn validate_envelope(
        self,
        envelope: &Envelope<model::OperatorRecord>,
    ) -> Result<ValidationState, ValidationError> {
        let state = self.validate_record(envelope.key_id.clone(), envelope)?;

        if let ValidationState::Initialized(initialized_state) = &state {
            if let Some(key) = initialized_state
                .entry_state
                .known_keys
                .get(&envelope.key_id)
            {
                model::OperatorRecord::verify(key, &envelope.content_bytes, &envelope.signature)?;
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
        envelope: &Envelope<model::OperatorRecord>,
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

    fn validate_record_hash(&self, record: &model::OperatorRecord) -> Result<(), ValidationError> {
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
                    return Err(ValidationError::RecordHashDoesNotMatch);
                }
                Ok(())
            }
        }
    }

    fn validate_record_version(
        &self,
        record: &model::OperatorRecord,
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
        record: &model::OperatorRecord,
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
        record: &model::OperatorRecord,
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
    pub hash_algorithm: hash::HashAlgorithm,
    /// The permissions associated with a given key_id
    pub permissions: HashMap<signing::KeyID, HashSet<model::Permission>>,
    /// The relevant known keys to this
    pub known_keys: HashMap<signing::KeyID, signing::PublicKey>,
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
        entry: &model::OperatorEntry,
    ) -> Result<Self, ValidationError> {
        match entry {
            model::OperatorEntry::Init {
                hash_algorithm,
                key: init_key,
            } => Ok(EntryValidationState {
                hash_algorithm: *hash_algorithm,
                permissions: HashMap::from([(key_id, HashSet::from([model::Permission::Commit]))]),
                known_keys: HashMap::from([(init_key.fingerprint(), init_key.clone())]),
            }),
            _ => Err(ValidationError::FirstEntryIsNotInit),
        }
    }

    pub fn validate_next(
        mut self,
        key_id: signing::KeyID,
        entry: &model::OperatorEntry,
    ) -> Result<EntryValidationState, ValidationError> {
        match entry {
            // Invalid re-initialization
            model::OperatorEntry::Init { .. } => Err(ValidationError::InitialEntryAfterBeginning),

            model::OperatorEntry::GrantFlat { key, permission } => {
                // Check that the current key has the permission they're trying to revoke
                self.check_key_permission(key_id, *permission)?;

                let grant_key_id = key.fingerprint();
                self.known_keys.insert(grant_key_id.clone(), key.clone());

                match self.permissions.entry(grant_key_id) {
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().insert(*permission);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(HashSet::from([*permission]));
                    }
                };

                Ok(self)
            }

            model::OperatorEntry::RevokeFlat { key_id, permission } => {
                // Check that the current key has the permission they're trying to revoke
                self.check_key_permission(key_id.clone(), *permission)?;

                match self.permissions.entry(key_id.clone()) {
                    Entry::Occupied(mut entry) => {
                        let permissions_set = entry.get_mut();
                        if !permissions_set.contains(permission) {
                            return Err(ValidationError::PermissionNotFoundToRevoke {
                                permission: *permission,
                                key_id: key_id.clone(),
                            });
                        }
                        entry.get_mut().remove(permission);
                    }
                    Entry::Vacant(_) => {
                        return Err(ValidationError::PermissionNotFoundToRevoke {
                            permission: *permission,
                            key_id: key_id.clone(),
                        })
                    }
                };
                Ok(self)
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
        let record = model::OperatorRecord {
            prev: None,
            version: 0,
            timestamp,
            entries: vec![model::OperatorEntry::Init {
                hash_algorithm: HashAlgorithm::Sha256,
                key: alice_pub.clone(),
            }],
        };

        let envelope = match Envelope::signed_contents(&alice_priv, record) {
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
                    HashSet::from([model::Permission::Commit]),
                )]),
                known_keys: HashMap::from([(alice_pub.fingerprint(), alice_pub)]),
            },
        });

        assert_eq!(expected_state, validation_state);
    }
}
