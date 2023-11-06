use super::{DataStore, DataStoreError};
use futures::Stream;
use indexmap::IndexMap;
use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::RwLock;
use warg_crypto::{hash::AnyHash, Signable};
use warg_protocol::{
    operator,
    package::{self, PackageEntry},
    registry::{
        LogId, LogLeaf, PackageId, RecordId, RegistryIndex, RegistryLen, TimestampedCheckpoint,
    },
    ProtoEnvelope, PublishedProtoEnvelope, SerdeEnvelope,
};

struct Entry<R> {
    registry_index: RegistryIndex,
    record_content: ProtoEnvelope<R>,
}

struct Log<V, R> {
    validator: V,
    entries: Vec<Entry<R>>,
}

impl<V, R> Default for Log<V, R>
where
    V: Default,
{
    fn default() -> Self {
        Self {
            validator: V::default(),
            entries: Vec::new(),
        }
    }
}

struct Record {
    /// Index in the log's entries.
    index: usize,
    /// Index in the registry's log.
    registry_index: RegistryIndex,
}

enum PendingRecord {
    Operator {
        record: Option<ProtoEnvelope<operator::OperatorRecord>>,
    },
    Package {
        record: Option<ProtoEnvelope<package::PackageRecord>>,
        missing: HashSet<AnyHash>,
    },
}

enum RejectedRecord {
    Operator {
        record: ProtoEnvelope<operator::OperatorRecord>,
        reason: String,
    },
    Package {
        record: ProtoEnvelope<package::PackageRecord>,
        reason: String,
    },
}

enum RecordStatus {
    Pending(PendingRecord),
    Rejected(RejectedRecord),
    Validated(Record),
}

#[derive(Default)]
struct State {
    operators: HashMap<LogId, Log<operator::LogState, operator::OperatorRecord>>,
    packages: HashMap<LogId, Log<package::LogState, package::PackageRecord>>,
    package_ids: HashMap<LogId, Option<PackageId>>,
    checkpoints: IndexMap<RegistryLen, SerdeEnvelope<TimestampedCheckpoint>>,
    records: HashMap<LogId, HashMap<RecordId, RecordStatus>>,
    log_leafs: HashMap<RegistryIndex, LogLeaf>,
}

/// Represents an in-memory data store.
///
/// Data is not persisted between restarts of the server.
///
/// Note: this is mainly used for testing, so it is not very efficient as
/// it shares a single RwLock for all operations.
pub struct MemoryDataStore(Arc<RwLock<State>>);

impl MemoryDataStore {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(State::default())))
    }
}

impl Default for MemoryDataStore {
    fn default() -> Self {
        Self::new()
    }
}

#[axum::async_trait]
impl DataStore for MemoryDataStore {
    async fn get_all_checkpoints(
        &self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<TimestampedCheckpoint, DataStoreError>> + Send>>,
        DataStoreError,
    > {
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn get_all_validated_records(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LogLeaf, DataStoreError>> + Send>>, DataStoreError>
    {
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn get_log_leafs_starting_with_registry_index(
        &self,
        starting_index: RegistryIndex,
        limit: Option<usize>,
    ) -> Result<Vec<(RegistryIndex, LogLeaf)>, DataStoreError> {
        let state = self.0.read().await;

        let limit = limit.unwrap_or(state.log_leafs.len() - starting_index);

        let mut leafs = Vec::with_capacity(limit);
        for entry in starting_index..starting_index + limit {
            match state.log_leafs.get(&entry) {
                Some(log_leaf) => leafs.push((entry, log_leaf.clone())),
                None => break,
            }
        }

        Ok(leafs)
    }

    async fn get_log_leafs_with_registry_index(
        &self,
        entries: &[RegistryIndex],
    ) -> Result<Vec<LogLeaf>, DataStoreError> {
        let state = self.0.read().await;

        let mut leafs = Vec::with_capacity(entries.len());
        for entry in entries {
            match state.log_leafs.get(entry) {
                Some(log_leaf) => leafs.push(log_leaf.clone()),
                None => return Err(DataStoreError::LogLeafNotFound(*entry)),
            }
        }

        Ok(leafs)
    }

    async fn get_package_ids(
        &self,
        log_ids: &[LogId],
    ) -> Result<HashMap<LogId, Option<PackageId>>, DataStoreError> {
        let state = self.0.read().await;

        log_ids
            .iter()
            .map(|log_id| {
                if let Some(opt_package_id) = state.package_ids.get(log_id) {
                    Ok((log_id.clone(), opt_package_id.clone()))
                } else {
                    Err(DataStoreError::LogNotFound(log_id.clone()))
                }
            })
            .collect::<Result<HashMap<LogId, Option<PackageId>>, _>>()
    }

    async fn store_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        record: &ProtoEnvelope<operator::OperatorRecord>,
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;
        let prev = state.records.entry(log_id.clone()).or_default().insert(
            record_id.clone(),
            RecordStatus::Pending(PendingRecord::Operator {
                record: Some(record.clone()),
            }),
        );

        assert!(prev.is_none());
        Ok(())
    }

    async fn reject_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        reason: &str,
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;

        let status = state
            .records
            .get_mut(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get_mut(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        let record = match status {
            RecordStatus::Pending(PendingRecord::Operator { record }) => record.take().unwrap(),
            _ => return Err(DataStoreError::RecordNotPending(record_id.clone())),
        };

        *status = RecordStatus::Rejected(RejectedRecord::Operator {
            record,
            reason: reason.to_string(),
        });

        Ok(())
    }

    async fn commit_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        registry_index: RegistryIndex,
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;

        let State {
            operators,
            records,
            log_leafs,
            ..
        } = &mut *state;

        let status = records
            .get_mut(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get_mut(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        match status {
            RecordStatus::Pending(PendingRecord::Operator { record }) => {
                let record = record.take().unwrap();
                let log = operators.entry(log_id.clone()).or_default();
                match log
                    .validator
                    .validate(&record)
                    .map_err(DataStoreError::from)
                {
                    Ok(_) => {
                        let index = log.entries.len();
                        log.entries.push(Entry {
                            registry_index,
                            record_content: record,
                        });
                        *status = RecordStatus::Validated(Record {
                            index,
                            registry_index,
                        });
                        log_leafs.insert(
                            registry_index,
                            LogLeaf {
                                log_id: log_id.clone(),
                                record_id: record_id.clone(),
                            },
                        );
                        Ok(())
                    }
                    Err(e) => {
                        *status = RecordStatus::Rejected(RejectedRecord::Operator {
                            record,
                            reason: e.to_string(),
                        });
                        Err(e)
                    }
                }
            }
            _ => Err(DataStoreError::RecordNotPending(record_id.clone())),
        }
    }

    async fn store_package_record(
        &self,
        log_id: &LogId,
        package_id: &PackageId,
        record_id: &RecordId,
        record: &ProtoEnvelope<package::PackageRecord>,
        missing: &HashSet<&AnyHash>,
    ) -> Result<(), DataStoreError> {
        // Ensure the set of missing hashes is a subset of the record contents.
        debug_assert!({
            use warg_protocol::Record;
            let contents = record.as_ref().contents();
            missing.is_subset(&contents)
        });

        let mut state = self.0.write().await;
        let prev = state.records.entry(log_id.clone()).or_default().insert(
            record_id.clone(),
            RecordStatus::Pending(PendingRecord::Package {
                record: Some(record.clone()),
                missing: missing.iter().map(|&d| d.clone()).collect(),
            }),
        );
        state
            .package_ids
            .insert(log_id.clone(), Some(package_id.clone()));

        assert!(prev.is_none());
        Ok(())
    }

    async fn reject_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        reason: &str,
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;

        let status = state
            .records
            .get_mut(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get_mut(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        let record = match status {
            RecordStatus::Pending(PendingRecord::Package { record, .. }) => record.take().unwrap(),
            _ => return Err(DataStoreError::RecordNotPending(record_id.clone())),
        };

        *status = RecordStatus::Rejected(RejectedRecord::Package {
            record,
            reason: reason.to_string(),
        });

        Ok(())
    }

    async fn commit_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        registry_index: RegistryIndex,
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;

        let State {
            packages,
            records,
            log_leafs,
            ..
        } = &mut *state;

        let status = records
            .get_mut(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get_mut(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        match status {
            RecordStatus::Pending(PendingRecord::Package { record, .. }) => {
                let record = record.take().unwrap();
                let log = packages.entry(log_id.clone()).or_default();
                match log
                    .validator
                    .validate(&record)
                    .map_err(DataStoreError::from)
                {
                    Ok(_) => {
                        let index = log.entries.len();
                        log.entries.push(Entry {
                            registry_index,
                            record_content: record,
                        });
                        *status = RecordStatus::Validated(Record {
                            index,
                            registry_index,
                        });
                        log_leafs.insert(
                            registry_index,
                            LogLeaf {
                                log_id: log_id.clone(),
                                record_id: record_id.clone(),
                            },
                        );
                        Ok(())
                    }
                    Err(e) => {
                        *status = RecordStatus::Rejected(RejectedRecord::Package {
                            record,
                            reason: e.to_string(),
                        });
                        Err(e)
                    }
                }
            }
            _ => Err(DataStoreError::RecordNotPending(record_id.clone())),
        }
    }

    async fn is_content_missing(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        digest: &AnyHash,
    ) -> Result<bool, DataStoreError> {
        let state = self.0.read().await;
        let log = state
            .records
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        let status = log
            .get(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        match status {
            RecordStatus::Pending(PendingRecord::Operator { .. }) => {
                // Operator records have no content
                Ok(false)
            }
            RecordStatus::Pending(PendingRecord::Package { missing, .. }) => {
                Ok(missing.contains(digest))
            }
            _ => return Err(DataStoreError::RecordNotPending(record_id.clone())),
        }
    }

    async fn set_content_present(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        digest: &AnyHash,
    ) -> Result<bool, DataStoreError> {
        let mut state = self.0.write().await;
        let log = state
            .records
            .get_mut(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        let status = log
            .get_mut(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        match status {
            RecordStatus::Pending(PendingRecord::Operator { .. }) => {
                // Operator records have no content, so conceptually already present
                Ok(false)
            }
            RecordStatus::Pending(PendingRecord::Package { missing, .. }) => {
                if missing.is_empty() {
                    return Ok(false);
                }

                // Return true if this was the last missing content
                missing.remove(digest);
                Ok(missing.is_empty())
            }
            _ => return Err(DataStoreError::RecordNotPending(record_id.clone())),
        }
    }

    async fn store_checkpoint(
        &self,
        _checkpoint_id: &AnyHash,
        ts_checkpoint: SerdeEnvelope<TimestampedCheckpoint>,
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;

        state
            .checkpoints
            .insert(ts_checkpoint.as_ref().checkpoint.log_length, ts_checkpoint);

        Ok(())
    }

    async fn get_latest_checkpoint(
        &self,
    ) -> Result<SerdeEnvelope<TimestampedCheckpoint>, DataStoreError> {
        let state = self.0.read().await;
        let checkpoint = state.checkpoints.values().last().unwrap();
        Ok(checkpoint.clone())
    }

    async fn get_checkpoint(
        &self,
        log_length: RegistryLen,
    ) -> Result<SerdeEnvelope<TimestampedCheckpoint>, DataStoreError> {
        let state = self.0.read().await;
        let checkpoint = state
            .checkpoints
            .get(&log_length)
            .ok_or_else(|| DataStoreError::CheckpointNotFound(log_length))?;
        Ok(checkpoint.clone())
    }

    async fn get_operator_records(
        &self,
        log_id: &LogId,
        registry_log_length: RegistryLen,
        since: Option<&RecordId>,
        limit: u16,
    ) -> Result<Vec<PublishedProtoEnvelope<operator::OperatorRecord>>, DataStoreError> {
        let state = self.0.read().await;

        let log = state
            .operators
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        if !state.checkpoints.contains_key(&registry_log_length) {
            return Err(DataStoreError::CheckpointNotFound(registry_log_length));
        };

        let start_log_idx = match since {
            Some(since) => match &state.records[log_id][since] {
                RecordStatus::Validated(record) => record.index + 1,
                _ => unreachable!(),
            },
            None => 0,
        };

        Ok(log
            .entries
            .iter()
            .skip(start_log_idx)
            .take_while(|entry| entry.registry_index < registry_log_length)
            .map(|entry| PublishedProtoEnvelope {
                envelope: entry.record_content.clone(),
                registry_index: entry.registry_index,
            })
            .take(limit as usize)
            .collect())
    }

    async fn get_package_records(
        &self,
        log_id: &LogId,
        registry_log_length: RegistryLen,
        since: Option<&RecordId>,
        limit: u16,
    ) -> Result<Vec<PublishedProtoEnvelope<package::PackageRecord>>, DataStoreError> {
        let state = self.0.read().await;

        let log = state
            .packages
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        if !state.checkpoints.contains_key(&registry_log_length) {
            return Err(DataStoreError::CheckpointNotFound(registry_log_length));
        };

        let start_log_idx = match since {
            Some(since) => match &state.records[log_id][since] {
                RecordStatus::Validated(record) => record.index + 1,
                _ => unreachable!(),
            },
            None => 0,
        };

        Ok(log
            .entries
            .iter()
            .skip(start_log_idx)
            .take_while(|entry| entry.registry_index < registry_log_length)
            .map(|entry| PublishedProtoEnvelope {
                envelope: entry.record_content.clone(),
                registry_index: entry.registry_index,
            })
            .take(limit as usize)
            .collect())
    }

    async fn get_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<super::Record<operator::OperatorRecord>, DataStoreError> {
        let state = self.0.read().await;
        let status = state
            .records
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        let (status, envelope, registry_index) = match status {
            RecordStatus::Pending(PendingRecord::Operator { record, .. }) => {
                (super::RecordStatus::Pending, record.clone().unwrap(), None)
            }
            RecordStatus::Rejected(RejectedRecord::Operator { record, reason }) => (
                super::RecordStatus::Rejected(reason.into()),
                record.clone(),
                None,
            ),
            RecordStatus::Validated(r) => {
                let log = state
                    .operators
                    .get(log_id)
                    .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

                let published_length = state
                    .checkpoints
                    .last()
                    .map(|(_, c)| c.as_ref().checkpoint.log_length)
                    .unwrap_or_default();

                (
                    if r.registry_index < published_length {
                        super::RecordStatus::Published
                    } else {
                        super::RecordStatus::Validated
                    },
                    log.entries[r.index].record_content.clone(),
                    Some(r.registry_index),
                )
            }
            _ => return Err(DataStoreError::RecordNotFound(record_id.clone())),
        };

        Ok(super::Record {
            status,
            envelope,
            registry_index,
        })
    }

    async fn get_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<super::Record<package::PackageRecord>, DataStoreError> {
        let state = self.0.read().await;
        let status = state
            .records
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        let (status, envelope, registry_index) = match status {
            RecordStatus::Pending(PendingRecord::Package { record, .. }) => {
                (super::RecordStatus::Pending, record.clone().unwrap(), None)
            }
            RecordStatus::Rejected(RejectedRecord::Package { record, reason }) => (
                super::RecordStatus::Rejected(reason.into()),
                record.clone(),
                None,
            ),
            RecordStatus::Validated(r) => {
                let log = state
                    .packages
                    .get(log_id)
                    .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

                let published_length = state
                    .checkpoints
                    .last()
                    .map(|(_, c)| c.as_ref().checkpoint.log_length)
                    .unwrap_or_default();

                (
                    if r.registry_index < published_length {
                        super::RecordStatus::Published
                    } else {
                        super::RecordStatus::Validated
                    },
                    log.entries[r.index].record_content.clone(),
                    Some(r.registry_index),
                )
            }
            _ => return Err(DataStoreError::RecordNotFound(record_id.clone())),
        };

        Ok(super::Record {
            status,
            envelope,
            registry_index,
        })
    }

    async fn verify_package_record_signature(
        &self,
        log_id: &LogId,
        record: &ProtoEnvelope<package::PackageRecord>,
    ) -> Result<(), DataStoreError> {
        let state = self.0.read().await;
        let key = match state
            .packages
            .get(log_id)
            .and_then(|log| log.validator.public_key(record.key_id()))
        {
            Some(key) => Some(key),
            None => match record.as_ref().entries.first() {
                Some(PackageEntry::Init { key, .. }) => Some(key),
                _ => return Err(DataStoreError::UnknownKey(record.key_id().clone())),
            },
        }
        .ok_or_else(|| DataStoreError::UnknownKey(record.key_id().clone()))?;

        package::PackageRecord::verify(key, record.content_bytes(), record.signature())
            .map_err(|_| DataStoreError::SignatureVerificationFailed)
    }

    #[cfg(feature = "debug")]
    async fn debug_list_package_ids(&self) -> anyhow::Result<Vec<PackageId>> {
        let state = self.0.read().await;
        Ok(state
            .package_ids
            .values()
            .filter_map(|opt_package_id| opt_package_id)
            .cloned()
            .collect())
    }
}
