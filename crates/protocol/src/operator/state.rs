use super::{model, OPERATOR_RECORD_VERSION};
use crate::registry::PackageName;
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

    #[error(
        "the namespace `{namespace}` is invalid; namespace must be a lowercased kebab case string"
    )]
    InvalidNamespace { namespace: String },

    #[error("the namespace `{namespace}` is already defined and cannot be redefined")]
    NamespaceAlreadyDefined { namespace: String },
}

/// The namespace definition.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct NamespaceDefinition {
    /// Namespace state.
    state: NamespaceState,
}

/// The namespace state for defining or importing from other registries.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NamespaceState {
    /// The namespace is defined for the registry to use for its own package logs.
    Defined,
    /// The namespace is imported from another registry.
    #[serde(rename_all = "camelCase")]
    Imported {
        /// The imported registry.
        registry: String,
    },
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
    /// The current head of the state.
    #[serde(skip_serializing_if = "Option::is_none")]
    head: Option<Head>,
    /// The permissions of each key.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    permissions: IndexMap<signing::KeyID, IndexSet<model::Permission>>,
    /// The keys known to the state.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    keys: IndexMap<signing::KeyID, signing::PublicKey>,
    /// The namespaces known to the state. The key is the namespace.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    namespaces: IndexMap<String, NamespaceDefinition>,
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
    /// Note that on failure, the log state is consumed to prevent
    /// invalid state from being used in future validations.
    pub fn validate(
        mut self,
        record: &ProtoEnvelope<model::OperatorRecord>,
    ) -> Result<Self, ValidationError> {
        self.validate_record(record)?;
        Ok(self)
    }

    /// Gets the public key of the given key id.
    ///
    /// Returns `None` if the key id is not recognized.
    pub fn public_key(&self, key_id: &signing::KeyID) -> Option<&signing::PublicKey> {
        self.keys.get(key_id)
    }

    /// Gets the namespace state.
    pub fn namespace_state(&self, namespace: &str) -> Option<&NamespaceState> {
        self.namespaces.get(namespace).map(|def| &def.state)
    }

    /// Checks the key has permission to sign checkpoints.
    pub fn key_has_permission_to_sign_checkpoints(&self, key_id: &signing::KeyID) -> bool {
        self.check_key_permissions(key_id, &[model::Permission::Commit])
            .is_ok()
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

        // Update the state head
        self.head = Some(Head {
            digest: RecordId::operator_record::<Sha256>(envelope),
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
                model::OperatorEntry::DefineNamespace { namespace } => {
                    self.validate_namespace(namespace, NamespaceState::Defined)?
                }
                model::OperatorEntry::ImportNamespace {
                    namespace,
                    registry,
                } => self.validate_namespace(
                    namespace,
                    NamespaceState::Imported {
                        registry: registry.to_string(),
                    },
                )?,
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
                .map(|set| set.swap_remove(permission))
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

    fn validate_namespace(
        &mut self,
        namespace: &str,
        state: NamespaceState,
    ) -> Result<(), ValidationError> {
        if !PackageName::is_valid_namespace(namespace) {
            return Err(ValidationError::InvalidNamespace {
                namespace: namespace.to_string(),
            });
        }

        if self.namespaces.contains_key(namespace) {
            // namespace is already defined
            Err(ValidationError::NamespaceAlreadyDefined {
                namespace: namespace.to_string(),
            })
        } else {
            // namespace is not defined
            self.namespaces
                .insert(namespace.to_string(), NamespaceDefinition { state });

            Ok(())
        }
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
}

impl crate::Validator for LogState {
    type Record = model::OperatorRecord;
    type Error = ValidationError;

    fn validate(self, record: &ProtoEnvelope<Self::Record>) -> Result<Self, Self::Error> {
        self.validate(record)
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
        let state = LogState::default();
        let state = state.validate(&envelope).unwrap();

        assert_eq!(
            state,
            LogState {
                head: Some(Head {
                    digest: RecordId::operator_record::<Sha256>(&envelope),
                    timestamp,
                }),
                algorithm: Some(HashAlgorithm::Sha256),
                permissions: IndexMap::from([(
                    alice_id.clone(),
                    IndexSet::from([
                        model::Permission::Commit,
                        model::Permission::DefineNamespace,
                        model::Permission::ImportNamespace
                    ]),
                )]),
                keys: IndexMap::from([(alice_id, alice_pub)]),
                namespaces: IndexMap::new(),
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
        let state = LogState::default();
        let state = state.validate(&envelope).unwrap();

        let expected = LogState {
            head: Some(Head {
                digest: RecordId::operator_record::<Sha256>(&envelope),
                timestamp,
            }),
            algorithm: Some(HashAlgorithm::Sha256),
            permissions: IndexMap::from([(
                alice_id.clone(),
                IndexSet::from([
                    model::Permission::Commit,
                    model::Permission::DefineNamespace,
                    model::Permission::ImportNamespace,
                ]),
            )]),
            keys: IndexMap::from([(alice_id, alice_pub)]),
            namespaces: IndexMap::new(),
        };

        assert_eq!(state, expected);

        let record = model::OperatorRecord {
            prev: Some(RecordId::operator_record::<Sha256>(&envelope)),
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
                // This entry is valid but should be rolled back since there is an invalid entry
                model::OperatorEntry::DefineNamespace {
                    namespace: "example-namespace".to_string(),
                },
            ],
        };

        let envelope =
            ProtoEnvelope::signed_contents(&alice_priv, record).expect("failed to sign envelope");

        // This validation should fail
        match state.validate(&envelope).unwrap_err() {
            ValidationError::PermissionNotFoundToRevoke { .. } => {}
            _ => panic!("expected a different error"),
        }
    }

    #[test]
    fn test_namespaces() {
        let (alice_pub, alice_priv) = generate_p256_pair();
        let alice_id = alice_pub.fingerprint();

        let timestamp = SystemTime::now();
        let record = model::OperatorRecord {
            prev: None,
            version: 0,
            timestamp,
            entries: vec![
                model::OperatorEntry::Init {
                    hash_algorithm: HashAlgorithm::Sha256,
                    key: alice_pub.clone(),
                },
                model::OperatorEntry::DefineNamespace {
                    namespace: "my-namespace".to_string(),
                },
                model::OperatorEntry::ImportNamespace {
                    namespace: "imported-namespace".to_string(),
                    registry: "registry.example.com".to_string(),
                },
            ],
        };

        let envelope =
            ProtoEnvelope::signed_contents(&alice_priv, record).expect("failed to sign envelope");
        let state = LogState::default();
        let state = state.validate(&envelope).unwrap();

        let expected = LogState {
            head: Some(Head {
                digest: RecordId::operator_record::<Sha256>(&envelope),
                timestamp,
            }),
            algorithm: Some(HashAlgorithm::Sha256),
            permissions: IndexMap::from([(
                alice_id.clone(),
                IndexSet::from([
                    model::Permission::Commit,
                    model::Permission::DefineNamespace,
                    model::Permission::ImportNamespace,
                ]),
            )]),
            keys: IndexMap::from([(alice_id, alice_pub)]),
            namespaces: IndexMap::from([
                (
                    "my-namespace".to_string(),
                    NamespaceDefinition {
                        state: NamespaceState::Defined,
                    },
                ),
                (
                    "imported-namespace".to_string(),
                    NamespaceDefinition {
                        state: NamespaceState::Imported {
                            registry: "registry.example.com".to_string(),
                        },
                    },
                ),
            ]),
        };

        assert_eq!(state, expected);

        {
            let record = model::OperatorRecord {
                prev: Some(RecordId::operator_record::<Sha256>(&envelope)),
                version: 0,
                timestamp: SystemTime::now(),
                entries: vec![
                    // This entry is valid
                    model::OperatorEntry::DefineNamespace {
                        namespace: "other-namespace".to_string(),
                    },
                    // This entry is not valid
                    model::OperatorEntry::ImportNamespace {
                        namespace: "my-namespace".to_string(),
                        registry: "registry.alternative.com".to_string(),
                    },
                ],
            };

            let envelope = ProtoEnvelope::signed_contents(&alice_priv, record)
                .expect("failed to sign envelope");

            // This validation should fail
            match state.clone().validate(&envelope).unwrap_err() {
                ValidationError::NamespaceAlreadyDefined { .. } => {}
                _ => panic!("expected a different error"),
            }
        }

        {
            let record = model::OperatorRecord {
                prev: Some(RecordId::operator_record::<Sha256>(&envelope)),
                version: 0,
                timestamp: SystemTime::now(),
                entries: vec![
                    // This entry is valid
                    model::OperatorEntry::DefineNamespace {
                        namespace: "other-namespace".to_string(),
                    },
                    // This entry is not valid
                    model::OperatorEntry::ImportNamespace {
                        namespace: "my-NAMESPACE".to_string(),
                        registry: "registry.alternative.com".to_string(),
                    },
                ],
            };

            let envelope = ProtoEnvelope::signed_contents(&alice_priv, record)
                .expect("failed to sign envelope");

            // This validation should fail
            match state.validate(&envelope).unwrap_err() {
                ValidationError::InvalidNamespace { .. } => {}
                _ => panic!("expected a different error"),
            }
        }
    }
}
