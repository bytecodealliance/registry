use super::{DataStore, DataStoreError, InitialLeaf, OperatorLogEntry, PackageLogEntry};
use futures::Stream;
use indexmap::IndexMap;
use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::RwLock;
use warg_api::content::ContentSource;
use warg_crypto::hash::DynHash;
use warg_protocol::{
    operator, package,
    registry::{LogId, LogLeaf, MapCheckpoint, RecordId},
    ProtoEnvelope, Record as _, SerdeEnvelope,
};

struct Log<V, R> {
    name: Option<String>,
    validator: V,
    entries: Vec<ProtoEnvelope<R>>,
    checkpoint_indices: Vec<usize>,
}

impl<V, R> Default for Log<V, R>
where
    V: Default,
{
    fn default() -> Self {
        Self {
            name: None,
            validator: V::default(),
            entries: Vec::new(),
            checkpoint_indices: Vec::new(),
        }
    }
}

struct Record {
    /// Index in the log's entries.
    index: usize,
    /// Index in the checkpoints map.
    checkpoint_index: Option<usize>,
    /// The related content sources (if there are any).
    sources: Vec<ContentSource>,
}

enum PendingRecord {
    Operator {
        record: Option<ProtoEnvelope<operator::OperatorRecord>>,
    },
    Package {
        name: String,
        record: Option<ProtoEnvelope<package::PackageRecord>>,
        sources: Vec<ContentSource>,
    },
}

enum RecordStatus {
    Pending(PendingRecord),
    Rejected(String),
    Validated(Record),
}

#[derive(Default)]
struct State {
    operators: HashMap<LogId, Log<operator::Validator, operator::OperatorRecord>>,
    packages: HashMap<LogId, Log<package::Validator, package::PackageRecord>>,
    checkpoints: IndexMap<DynHash, SerdeEnvelope<MapCheckpoint>>,
    records: HashMap<LogId, HashMap<RecordId, RecordStatus>>,
}

fn get_records_before_checkpoint(indices: &[usize], checkpoint_index: usize) -> usize {
    indices
        .iter()
        .filter(|index| **index <= checkpoint_index)
        .count()
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
    async fn get_initial_leaves(
        &self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<InitialLeaf, DataStoreError>> + Send>>,
        DataStoreError,
    > {
        Ok(Box::pin(futures::stream::empty()))
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

        match state
            .records
            .get_mut(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get_mut(record_id)
        {
            Some(s @ RecordStatus::Pending(PendingRecord::Operator { .. })) => {
                *s = RecordStatus::Rejected(reason.to_string());
                Ok(())
            }
            _ => Err(DataStoreError::RecordNotFound(record_id.clone())),
        }
    }

    async fn validate_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;

        let State {
            operators, records, ..
        } = &mut *state;

        let status = records
            .get_mut(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get_mut(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        let res = match status {
            RecordStatus::Pending(PendingRecord::Operator { record }) => {
                let record = record.take().unwrap();
                let log = operators.entry(log_id.clone()).or_default();
                log.validator
                    .validate(&record)
                    .map(|_| {
                        let index = log.entries.len();
                        log.entries.push(record);
                        Record {
                            index,
                            checkpoint_index: None,
                            sources: Default::default(),
                        }
                    })
                    .map_err(Into::into)
            }
            _ => Err(DataStoreError::RecordNotPending(record_id.clone())),
        };

        match res {
            Ok(record) => {
                *status = RecordStatus::Validated(record);
                Ok(())
            }
            Err(e) => {
                *status = RecordStatus::Rejected(e.to_string());
                Err(e)
            }
        }
    }

    async fn store_package_record(
        &self,
        log_id: &LogId,
        name: &str,
        record_id: &RecordId,
        record: &ProtoEnvelope<package::PackageRecord>,
        sources: &[ContentSource],
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;
        let prev = state.records.entry(log_id.clone()).or_default().insert(
            record_id.clone(),
            RecordStatus::Pending(PendingRecord::Package {
                name: name.to_string(),
                record: Some(record.clone()),
                sources: sources.to_vec(),
            }),
        );

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

        match state
            .records
            .get_mut(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get_mut(record_id)
        {
            Some(s @ RecordStatus::Pending(PendingRecord::Package { .. })) => {
                *s = RecordStatus::Rejected(reason.to_string());
                Ok(())
            }
            _ => Err(DataStoreError::RecordNotFound(record_id.clone())),
        }
    }

    async fn validate_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;

        let State {
            packages, records, ..
        } = &mut *state;

        let status = records
            .get_mut(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?
            .get_mut(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        let res = match status {
            RecordStatus::Pending(PendingRecord::Package {
                name,
                record,
                sources,
            }) => {
                let record = record.take().unwrap();
                let log = packages.entry(log_id.clone()).or_default();
                log.name.get_or_insert_with(|| name.to_string());

                let needed = record.as_ref().contents();
                let provided = sources
                    .iter()
                    .map(|source| &source.digest)
                    .collect::<HashSet<_>>();

                if let Some(missing) = needed.difference(&provided).next() {
                    Err(DataStoreError::Rejection(format!(
                        "a content source for digest `{missing}` was not provided"
                    )))
                } else if let Some(extra) = provided.difference(&needed).next() {
                    Err(DataStoreError::Rejection(format!(
                        "a content source for digest `{extra}` was provided but not needed",
                    )))
                } else {
                    drop(needed);
                    log.validator
                        .validate(&record)
                        .map(|_| {
                            let index = log.entries.len();
                            log.entries.push(record);
                            Record {
                                index,
                                checkpoint_index: None,
                                sources: sources.to_vec(),
                            }
                        })
                        .map_err(Into::into)
                }
            }
            _ => Err(DataStoreError::RecordNotPending(record_id.clone())),
        };

        match res {
            Ok(record) => {
                *status = RecordStatus::Validated(record);
                Ok(())
            }
            Err(e) => {
                *status = RecordStatus::Rejected(e.to_string());
                Err(e)
            }
        }
    }

    async fn store_checkpoint(
        &self,
        checkpoint_id: &DynHash,
        checkpoint: SerdeEnvelope<MapCheckpoint>,
        participants: &[LogLeaf],
    ) -> Result<(), DataStoreError> {
        let mut state = self.0.write().await;

        let (index, prev) = state
            .checkpoints
            .insert_full(checkpoint_id.clone(), checkpoint);
        assert!(prev.is_none());

        for leaf in participants {
            if let Some(log) = state.operators.get_mut(&leaf.log_id) {
                log.checkpoint_indices.push(index);
            } else if let Some(log) = state.packages.get_mut(&leaf.log_id) {
                log.checkpoint_indices.push(index);
            } else {
                unreachable!("log not found");
            }

            match state
                .records
                .get_mut(&leaf.log_id)
                .unwrap()
                .get_mut(&leaf.record_id)
                .unwrap()
            {
                RecordStatus::Validated(record) => {
                    record.checkpoint_index = Some(index);
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    async fn get_latest_checkpoint(&self) -> Result<SerdeEnvelope<MapCheckpoint>, DataStoreError> {
        let state = self.0.read().await;
        let checkpoint = state.checkpoints.values().last().unwrap();
        Ok(checkpoint.clone())
    }

    async fn get_operator_records(
        &self,
        log_id: &LogId,
        root: &DynHash,
        since: Option<&RecordId>,
    ) -> Result<Vec<ProtoEnvelope<operator::OperatorRecord>>, DataStoreError> {
        let state = self.0.read().await;

        let log = state
            .operators
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        if let Some(checkpoint_index) = state.checkpoints.get_index_of(root) {
            let start = match since {
                Some(since) => match &state.records[log_id][since] {
                    RecordStatus::Validated(record) => record.index + 1,
                    _ => unreachable!(),
                },
                None => 0,
            };
            let end = get_records_before_checkpoint(&log.checkpoint_indices, checkpoint_index);
            Ok(log.entries[start..end].to_vec())
        } else {
            Err(DataStoreError::CheckpointNotFound(root.clone()))
        }
    }

    async fn get_package_records(
        &self,
        log_id: &LogId,
        root: &DynHash,
        since: Option<&RecordId>,
    ) -> Result<Vec<ProtoEnvelope<package::PackageRecord>>, DataStoreError> {
        let state = self.0.read().await;

        let log = state
            .packages
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        if let Some(checkpoint_index) = state.checkpoints.get_index_of(root) {
            let start = match since {
                Some(since) => match &state.records[log_id][since] {
                    RecordStatus::Validated(record) => record.index + 1,
                    _ => unreachable!(),
                },
                None => 0,
            };
            let end = get_records_before_checkpoint(&log.checkpoint_indices, checkpoint_index);
            Ok(log.entries[start..end].to_vec())
        } else {
            Err(DataStoreError::CheckpointNotFound(root.clone()))
        }
    }

    async fn get_record_status(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<super::RecordStatus, DataStoreError> {
        let state = self.0.read().await;
        let log = state
            .records
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        let status = log
            .get(record_id)
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        match status {
            RecordStatus::Pending(_) => Ok(super::RecordStatus::Pending),
            RecordStatus::Rejected(reason) => Ok(super::RecordStatus::Rejected(reason.clone())),
            RecordStatus::Validated(r) => Ok(if r.checkpoint_index.is_some() {
                super::RecordStatus::InCheckpoint
            } else {
                super::RecordStatus::Validated
            }),
        }
    }

    async fn get_operator_log_entry(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<OperatorLogEntry, DataStoreError> {
        let state = self.0.read().await;
        let statuses = state
            .records
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        match statuses.get(record_id) {
            Some(RecordStatus::Validated(r)) => {
                let log = state
                    .operators
                    .get(log_id)
                    .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

                Ok(OperatorLogEntry {
                    record: log.entries[r.index].clone(),
                    checkpoint: state.checkpoints[r.checkpoint_index.unwrap()].clone(),
                })
            }
            _ => Err(DataStoreError::RecordNotFound(record_id.clone())),
        }
    }

    async fn get_package_log_entry(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<PackageLogEntry, DataStoreError> {
        let state = self.0.read().await;
        let statuses = state
            .records
            .get(log_id)
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        match statuses.get(record_id) {
            Some(RecordStatus::Validated(r)) => {
                let log = state
                    .packages
                    .get(log_id)
                    .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

                Ok(PackageLogEntry {
                    record: log.entries[r.index].clone(),
                    sources: r.sources.clone(),
                    checkpoint: state.checkpoints[r.checkpoint_index.unwrap()].clone(),
                })
            }
            _ => Err(DataStoreError::RecordNotFound(record_id.clone())),
        }
    }
}
