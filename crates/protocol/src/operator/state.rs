use super::{model, OPERATOR_RECORD_VERSION};
use crate::registry::RecordId;
use crate::ProtoEnvelope;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;
use warg_crypto::hash::{HashAlgorithm, Sha256};
use warg_crypto::{signing, Signable};

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("the first entry of the log is not \"init\"")]
    FirstEntryIsNotInit,

    #[error("the initial record is empty and does not \"init\"")]
    InitialRecordDoesNotInit,

    #[error("the Key ID used to sign this envelope is not known to this operator log")]
    KeyIDNotRecognized { key_id: signing::KeyID },

    #[error("a second \"init\" entry was found")]
    InitialEntryAfterBeginning,

    #[error("the key with ID {key_id} did not have required permission {needed_permission}")]
    UnauthorizedAction {
        key_id: signing::KeyID,
        needed_permission: model::Permission,
    },

    #[error("attempted to remove permission {permission} from key {key_id} which did not have it")]
    PermissionNotFoundToRevoke {
        permission: model::Permission,
        key_id: signing::KeyID,
    },

    #[error("unable to verify signature: {0}")]
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

/// Information about the current head of the operator log.
///
/// A head is the last validated record digest and timestamp.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Head {
    /// The digest of the last validated record.
    pub digest: RecordId,
    /// The timestamp of the last validated record.
    #[serde(with = "crate::timestamp")]
    pub timestamp: SystemTime,
}

/// Calculated state for an operator log.
#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct LogState {
    /// The hash algorithm used by the operator log.
    /// This is `None` until the first (i.e. init) record is validated.
    #[serde(skip_serializing_if = "Option::is_none")]
    algorithm: Option<HashAlgorithm>,
    /// The current head of the validator.
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<Head>,
    /// The permissions of each key.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    permissions: IndexMap<signing::KeyID, IndexSet<model::Permission>>,
    /// The keys known to the validator.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    keys: IndexMap<signing::KeyID, signing::PublicKey>,
}

impl LogState {
    /// Create a new operator log state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the current head of the log.
    ///
    /// Returns `None` if no records have been validated yet.
    pub fn head(&self) -> &Option<Head> {
        &self.head
    }

    /// Validates an individual operator record.
    ///
    /// It is expected that `validate` is called in order of the
    /// records in the log.
    ///
    /// This operation is transactional: if any entry in the record
    /// fails to validate, the validator state will remain unchanged.
    pub fn validate(
        &mut self,
        record: &ProtoEnvelope<model::OperatorRecord>,
    ) -> Result<(), ValidationError> {
        let snapshot = self.snapshot();

        let result = self.validate_record(record);
        if result.is_err() {
            self.rollback(snapshot);
        }

        result
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
        envelope: &ProtoEnvelope<model::OperatorRecord>,
    ) -> Result<(), ValidationError> {
        let record = envelope.as_ref();

        // Validate previous hash
        self.validate_record_hash(record)?;

        // Validate version
        self.validate_record_version(record)?;

        // Validate timestamp
        self.validate_record_timestamp(record)?;

        // Validate entries
        self.validate_record_entries(envelope.key_id(), &record.entries)?;

        // At this point the digest algorithm must be set via an init entry
        let _algorithm = self
            .algorithm
            .ok_or(ValidationError::InitialRecordDoesNotInit)?;

        // Validate the envelope key id
        let key = self.keys.get(envelope.key_id()).ok_or_else(|| {
            ValidationError::KeyIDNotRecognized {
                key_id: envelope.key_id().clone(),
            }
        })?;

        // Validate the envelope signature
        model::OperatorRecord::verify(key, envelope.content_bytes(), envelope.signature())?;

        // Update the validator head
        self.head = Some(Head {
            digest: RecordId::operator_record::<Sha256>(envelope.content_bytes()),
            timestamp: record.timestamp,
        });

        Ok(())
    }

    fn validate_record_hash(&self, record: &model::OperatorRecord) -> Result<(), ValidationError> {
        match (&self.head, &record.prev) {
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
        record: &model::OperatorRecord,
    ) -> Result<(), ValidationError> {
        if record.version == OPERATOR_RECORD_VERSION {
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
        if let Some(head) = &self.head {
            if record.timestamp < head.timestamp {
                return Err(ValidationError::TimestampLowerThanPrevious);
            }
        }

        Ok(())
    }

    fn validate_record_entries(
        &mut self,
        signer_key_id: &signing::KeyID,
        entries: &[model::OperatorEntry],
    ) -> Result<(), ValidationError> {
        for entry in entries {
            if let Some(permission) = entry.required_permission() {
                self.check_key_permissions(signer_key_id, &[permission])?;
            }

            // Process an init entry specially
            if let model::OperatorEntry::Init {
                hash_algorithm,
                key,
            } = entry
            {
                self.validate_init_entry(signer_key_id, *hash_algorithm, key)?;
                continue;
            }

            // Must have seen an init entry by now
            if !self.initialized() {
                return Err(ValidationError::FirstEntryIsNotInit);
            }

            match entry {
                model::OperatorEntry::Init { .. } => unreachable!(), // handled above
                model::OperatorEntry::GrantFlat { key, permissions } => {
                    self.validate_grant_entry(signer_key_id, key, permissions)?
                }
                model::OperatorEntry::RevokeFlat {
                    key_id,
                    permissions,
                } => self.validate_revoke_entry(signer_key_id, key_id, permissions)?,
            }
        }

        Ok(())
    }

    fn validate_init_entry(
        &mut self,
        signer_key_id: &signing::KeyID,
        algorithm: HashAlgorithm,
        init_key: &signing::PublicKey,
    ) -> Result<(), ValidationError> {
        if self.initialized() {
            return Err(ValidationError::InitialEntryAfterBeginning);
        }

        assert!(self.permissions.is_empty());
        assert!(self.keys.is_empty());

        self.algorithm = Some(algorithm);
        self.permissions.insert(
            signer_key_id.clone(),
            IndexSet::from(model::Permission::all()),
        );
        self.keys.insert(init_key.fingerprint(), init_key.clone());

        Ok(())
    }

    fn validate_grant_entry(
        &mut self,
        signer_key_id: &signing::KeyID,
        key: &signing::PublicKey,
        permissions: &[model::Permission],
    ) -> Result<(), ValidationError> {
        // Check that the current key has the permission they're trying to grant
        self.check_key_permissions(signer_key_id, permissions)?;

        let grant_key_id = key.fingerprint();
        self.keys.insert(grant_key_id.clone(), key.clone());
        self.permissions
            .entry(grant_key_id)
            .or_default()
            .extend(permissions);

        Ok(())
    }

    fn validate_revoke_entry(
        &mut self,
        signer_key_id: &signing::KeyID,
        key_id: &signing::KeyID,
        permissions: &[model::Permission],
    ) -> Result<(), ValidationError> {
        // Check that the current key has the permission they're trying to revoke
        self.check_key_permissions(signer_key_id, permissions)?;

        for permission in permissions {
            if !self
                .permissions
                .get_mut(key_id)
                .map(|set| set.remove(permission))
                .unwrap_or(false)
            {
                return Err(ValidationError::PermissionNotFoundToRevoke {
                    permission: *permission,
                    key_id: key_id.clone(),
                });
            }
        }
        Ok(())
    }

    fn check_key_permissions(
        &self,
        key_id: &signing::KeyID,
        permissions: &[model::Permission],
    ) -> Result<(), ValidationError> {
        for permission in permissions {
            if !self
                .permissions
                .get(key_id)
                .map(|p| p.contains(permission))
                .unwrap_or(false)
            {
                return Err(ValidationError::UnauthorizedAction {
                    key_id: key_id.clone(),
                    needed_permission: *permission,
                });
            }
        }
        Ok(())
    }

    fn snapshot(&self) -> Snapshot {
        let Self {
            algorithm,
            head,
            permissions,
            keys,
        } = self;

        Snapshot {
            algorithm: *algorithm,
            head: head.clone(),
            permissions: permissions.len(),
            keys: keys.len(),
        }
    }

    fn rollback(&mut self, snapshot: Snapshot) {
        let Snapshot {
            algorithm,
            head,
            permissions,
            keys,
        } = snapshot;

        self.algorithm = algorithm;
        self.head = head;
        self.permissions.truncate(permissions);
        self.keys.truncate(keys);
    }
}

impl crate::Validator for LogState {
    type Record = model::OperatorRecord;
    type Error = ValidationError;

    fn validate(&mut self, record: &ProtoEnvelope<Self::Record>) -> Result<(), Self::Error> {
        self.validate(record)
    }
}

struct Snapshot {
    algorithm: Option<HashAlgorithm>,
    head: Option<Head>,
    permissions: usize,
    keys: usize,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use warg_crypto::signing::generate_p256_pair;

    use std::time::SystemTime;
    use warg_crypto::hash::HashAlgorithm;

    #[test]
    fn test_validate_base_log() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let alice_id = alice_pub.fingerprint();

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

        let envelope =
            ProtoEnvelope::signed_contents(&alice_priv, record).expect("failed to sign envelope");
        let mut validator = LogState::default();
        validator.validate(&envelope).unwrap();

        assert_eq!(
            validator,
            LogState {
                head: Some(Head {
                    digest: RecordId::operator_record::<Sha256>(envelope.content_bytes()),
                    timestamp,
                }),
                algorithm: Some(HashAlgorithm::Sha256),
                permissions: IndexMap::from([(
                    alice_id.clone(),
                    IndexSet::from([model::Permission::Commit]),
                )]),
                keys: IndexMap::from([(alice_id, alice_pub)]),
            }
        );
    }

    #[test]
    fn test_rollback() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let alice_id = alice_pub.fingerprint();
        let (bob_pub, _) = generate_p256_pair();

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

        let envelope =
            ProtoEnvelope::signed_contents(&alice_priv, record).expect("failed to sign envelope");
        let mut validator = LogState::default();
        validator.validate(&envelope).unwrap();

        let expected = LogState {
            head: Some(Head {
                digest: RecordId::operator_record::<Sha256>(envelope.content_bytes()),
                timestamp,
            }),
            algorithm: Some(HashAlgorithm::Sha256),
            permissions: IndexMap::from([(
                alice_id.clone(),
                IndexSet::from([model::Permission::Commit]),
            )]),
            keys: IndexMap::from([(alice_id, alice_pub)]),
        };

        assert_eq!(validator, expected);

        let record = model::OperatorRecord {
            prev: Some(RecordId::operator_record::<Sha256>(
                envelope.content_bytes(),
            )),
            version: 0,
            timestamp: SystemTime::now(),
            entries: vec![
                // This entry is valid
                model::OperatorEntry::GrantFlat {
                    key: bob_pub,
                    permissions: vec![model::Permission::Commit],
                },
                // This entry is not valid
                model::OperatorEntry::RevokeFlat {
                    key_id: "not-valid".to_string().into(),
                    permissions: vec![model::Permission::Commit],
                },
            ],
        };

        let envelope =
            ProtoEnvelope::signed_contents(&alice_priv, record).expect("failed to sign envelope");

        // This validation should fail and the validator state should remain unchanged
        match validator.validate(&envelope).unwrap_err() {
            ValidationError::PermissionNotFoundToRevoke { .. } => {}
            _ => panic!("expected a different error"),
        }

        // The validator should not have changed
        assert_eq!(validator, expected);
    }
}
