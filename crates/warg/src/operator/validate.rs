use super::{model, OPERATOR_RECORD_VERSION};
use crate::ProtoEnvelope;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;
use warg_crypto::hash::{DynHash, HashAlgorithm};
use warg_crypto::{signing, Signable};

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("The first entry of the log is not \"init\"")]
    FirstEntryIsNotInit,

    #[error("The initial record is empty and does not \"init\"")]
    InitialRecordDoesNotInit,

    #[error("The Key ID used to sign this envelope is not known to this operator log")]
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

/// Information about the current validation root of the operator log.
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

/// A validator for operator records.
#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validator {
    /// The hash algorithm used by the operator log.
    /// This is `None` until the first (i.e. init) record is validated.
    #[serde(skip_serializing_if = "Option::is_none")]
    algorithm: Option<HashAlgorithm>,
    /// The current root of the validator.
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<Root>,
    /// The permissions of each key.
    #[serde(skip_serializing_if = "IndexMap::is_empty", default)]
    permissions: IndexMap<KeyIndex, IndexSet<model::Permission>>,
    /// The keys known to the validator.
    #[serde(skip_serializing_if = "IndexMap::is_empty", default)]
    keys: IndexMap<signing::KeyID, signing::PublicKey>,
}

impl Validator {
    /// Create a new operator log validator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the current root of the validator.
    ///
    /// Returns `None` if no records have been validated yet.
    pub fn root(&self) -> &Option<Root> {
        &self.root
    }

    /// Validates an individual operator record.
    ///
    /// It is expected that `validate` is called in order of the
    /// records in the log.
    pub fn validate(
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
        model::OperatorRecord::verify(key, envelope.content_bytes(), envelope.signature())?;

        // Update the validator root
        self.root = Some(Root {
            digest: algorithm.digest(envelope.content_bytes()),
            timestamp: record.timestamp,
        });

        Ok(())
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

    fn validate_record_hash(&self, record: &model::OperatorRecord) -> Result<(), ValidationError> {
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
        entries: &[model::OperatorEntry],
    ) -> Result<(), ValidationError> {
        let mut signer_key_index = None;
        for entry in entries {
            // Process an init entry specially
            if let model::OperatorEntry::Init {
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
                model::OperatorEntry::Init { .. } => unreachable!(), // handled above
                model::OperatorEntry::GrantFlat { key, permission } => {
                    self.validate_grant_entry(signer_key_index.unwrap(), key, *permission)?
                }
                model::OperatorEntry::RevokeFlat { key_id, permission } => {
                    self.validate_revoke_entry(signer_key_index.unwrap(), key_id, *permission)?
                }
            }
        }

        Ok(())
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
                    IndexSet::from([model::Permission::Commit]),
                )]),
                keys: IndexMap::from([(alice_id, alice_pub)]),
            }
        );
    }
}
