use super::{
    data::log::LogData,
    transparency::{VerifiableLog, VerifiableMap},
    MapData,
};
use crate::{
    datastore::{DataStore, DataStoreError, InitialLeaf, PackageLogEntry, RecordStatus},
    services::{data, transparency},
};
use anyhow::Result;
use futures::StreamExt;
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use thiserror::Error;
use tokio::{
    sync::{
        mpsc::{self, Sender},
        oneshot, RwLock,
    },
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use warg_api::content::ContentSource;
use warg_crypto::{
    hash::{DynHash, Hash, HashAlgorithm, Sha256},
    signing::PrivateKey,
};
use warg_protocol::{
    operator, package,
    registry::{LogId, LogLeaf, MapCheckpoint, MapLeaf, RecordId},
    ProtoEnvelope, SerdeEnvelope,
};
use warg_transparency::log::LogBuilder;

fn init_envelope(signing_key: &PrivateKey) -> ProtoEnvelope<operator::OperatorRecord> {
    let init_record = operator::OperatorRecord {
        prev: None,
        version: 0,
        timestamp: SystemTime::now(),
        entries: vec![operator::OperatorEntry::Init {
            hash_algorithm: HashAlgorithm::Sha256,
            key: signing_key.public_key(),
        }],
    };
    ProtoEnvelope::signed_contents(signing_key, init_record).unwrap()
}

#[derive(Debug, Error)]
pub enum CoreServiceError {
    #[error("package log `{0}` was not found")]
    PackageNotFound(String),

    #[error(transparent)]
    DataStore(#[from] DataStoreError),
}

impl CoreServiceError {
    fn with_package_name(self, name: &str) -> Self {
        match self {
            Self::DataStore(DataStoreError::LogNotFound(_)) => {
                Self::PackageNotFound(name.to_string())
            }
            e => e,
        }
    }
}

/// Used to stop the core service.
pub struct StopHandle {
    token: CancellationToken,
    join: Vec<JoinHandle<()>>,
}

impl StopHandle {
    /// Stops the core service and waits for all tasks to complete.
    pub async fn stop(self) {
        self.token.cancel();
        futures::future::join_all(self.join).await;
    }
}

#[derive(Debug)]
struct NewPackageRecord {
    package_name: String,
    record: ProtoEnvelope<package::PackageRecord>,
    content_sources: Vec<ContentSource>,
    response: oneshot::Sender<Result<(), CoreServiceError>>,
}

#[derive(Default)]
struct InitializationData {
    log: VerifiableLog,
    map: VerifiableMap,
    log_data: LogData,
    map_data: MapData,
    leaves: Vec<LogLeaf>,
}

pub struct CoreService {
    log: Arc<RwLock<LogData>>,
    map: Arc<RwLock<MapData>>,
    new_record_tx: Sender<NewPackageRecord>,
    store: Box<dyn DataStore>,
}

impl CoreService {
    /// Spawn the core service with the given operator signing key.
    pub async fn spawn(
        signing_key: PrivateKey,
        store: Box<dyn DataStore>,
        checkpoint_interval: Duration,
    ) -> Result<(Arc<Self>, StopHandle), CoreServiceError> {
        let data = Self::initialize(&signing_key, store.as_ref()).await?;
        let token = CancellationToken::new();
        let (log_tx, log_rx) = mpsc::channel(4);

        // Spawn the transparency service
        let transparency = transparency::spawn(transparency::Input {
            token: token.clone(),
            checkpoint_interval,
            log: data.log,
            map: data.map,
            leaves: data.leaves,
            signing_key,
            log_rx,
        });

        // Spawn the data service
        let data = data::spawn(data::Input {
            token: token.clone(),
            log_data: data.log_data,
            log_rx: transparency.log_rx,
            map_data: data.map_data,
            map_rx: transparency.map_rx,
        });

        let (new_record_tx, mut new_record_rx) = mpsc::channel(4);
        let core = Arc::new(Self {
            log: data.log_data,
            map: data.map_data,
            new_record_tx,
            store,
        });

        let spawn_token = token.clone();
        let task_core = core.clone();
        let mut signatures = transparency.signature_rx;
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = spawn_token.cancelled() => {
                        break;
                    }
                    message = new_record_rx.recv() => {
                        if let Some(NewPackageRecord {
                            package_name,
                            record,
                            content_sources,
                            response,
                        }) = message {
                            let log_id = LogId::package_log::<Sha256>(&package_name);
                            let record_id = RecordId::package_record::<Sha256>(&record);
                            match task_core.store.store_package_record(&log_id, &package_name, &record_id, &record, &content_sources).await {
                                Ok(()) => {
                                    // Record saved successfully, so notify the client that the record is processing
                                    response.send(Ok(())).unwrap();

                                    // TODO: perform all policy checks on the record here

                                    // Accept the package record
                                    match task_core.store.accept_package_record(&log_id, &record_id).await {
                                        Ok(()) => {
                                            // Send the record to the transparency service to be included in the next checkpoint
                                            let leaf = LogLeaf { log_id, record_id };
                                            log_tx.send(leaf).await.unwrap();
                                        }
                                        Err(e) => match e {
                                            DataStoreError::Rejection(_)
                                            | DataStoreError::OperatorValidationFailed(_)
                                            | DataStoreError::PackageValidationFailed(_) => {
                                                // The record failed to validate and was rejected; do not include it in the next checkpoint
                                            }
                                            e => {
                                                // TODO: this should be made more robust with a proper reliable message
                                                // queue with retry logic
                                                tracing::error!("failed to accept package record `{record_id}`: {e}");
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    response.send(Err(e.into())).unwrap();
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    signature = signatures.recv() => {
                        if let Some(signature) = signature {
                            let checkpoint_id = Hash::<Sha256>::of(signature.envelope.as_ref()).into();
                            task_core.store.store_checkpoint(&checkpoint_id, signature.envelope, &signature.leaves).await.unwrap();
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        tracing::debug!("core service is running");

        let join = vec![
            transparency.log_handle,
            transparency.map_handle,
            transparency.sign_handle,
            data.log_handle,
            data.map_handle,
            handle,
        ];

        Ok((core, StopHandle { token, join }))
    }

    /// Get the log data associated with the core service.
    pub fn log_data(&self) -> &Arc<RwLock<LogData>> {
        &self.log
    }

    /// Get the map data associated with the core service.
    pub fn map_data(&self) -> &Arc<RwLock<MapData>> {
        &self.map
    }

    async fn initialize(
        signing_key: &PrivateKey,
        store: &dyn DataStore,
    ) -> Result<InitializationData, CoreServiceError> {
        tracing::debug!("initializing core service");
        let mut data = InitializationData::default();
        let mut last_checkpoint = None;
        let mut initial = store.get_initial_leaves().await?;
        while let Some(res) = initial.next().await {
            let InitialLeaf { leaf, checkpoint } = res?;
            data.log.push(&leaf);

            data.map = data.map.insert(
                leaf.log_id.clone(),
                MapLeaf {
                    record_id: leaf.record_id.clone(),
                },
            );

            match checkpoint {
                Some(checkpoint) => {
                    if last_checkpoint.as_ref() != Some(&checkpoint) {
                        data.map_data.insert(data.map.clone());
                        last_checkpoint = Some(checkpoint);
                    }
                }
                None => data.leaves.push(leaf.clone()),
            }

            data.log_data.push(leaf);
        }

        if data.log.is_empty() {
            return Self::init_operator_log(signing_key, store).await;
        }

        tracing::debug!("core service initialized");
        Ok(data)
    }

    async fn init_operator_log(
        signing_key: &PrivateKey,
        store: &dyn DataStore,
    ) -> Result<InitializationData, CoreServiceError> {
        let init = init_envelope(signing_key);
        let log_id = LogId::operator_log::<Sha256>();
        let record_id = RecordId::operator_record::<Sha256>(&init);

        store
            .store_operator_record(&log_id, &record_id, &init)
            .await?;

        // TODO: ensure the operator record passes all policy checks

        store.accept_operator_record(&log_id, &record_id).await?;

        let leaf = LogLeaf { log_id, record_id };
        let mut data = InitializationData::default();
        data.log.push(&leaf);

        data.map = data.map.insert(
            leaf.log_id.clone(),
            MapLeaf {
                record_id: leaf.record_id.clone(),
            },
        );

        data.log_data.push(leaf.clone());
        data.map_data.insert(data.map.clone());

        let checkpoint = data.log.checkpoint();
        let log_root: DynHash = checkpoint.root().into();
        let log_length = checkpoint.length() as u32;

        let checkpoint = MapCheckpoint {
            log_root,
            log_length,
            map_root: data.map.root().clone().into(),
        };

        let checkpoint = SerdeEnvelope::signed_contents(signing_key, checkpoint).unwrap();
        let checkpoint_id = Hash::<Sha256>::of(checkpoint.as_ref()).into();
        store
            .store_checkpoint(&checkpoint_id, checkpoint, &[leaf])
            .await?;

        Ok(data)
    }
}

impl CoreService {
    /// Submits a package record to be processed.
    pub async fn submit_package_record(
        &self,
        name: &str,
        record: ProtoEnvelope<package::PackageRecord>,
        content_sources: Vec<ContentSource>,
    ) -> Result<(), CoreServiceError> {
        let (tx, rx) = oneshot::channel();
        self.new_record_tx
            .send(NewPackageRecord {
                package_name: name.to_string(),
                record,
                content_sources,
                response: tx,
            })
            .await
            .unwrap();

        rx.await.unwrap().map_err(|e| e.with_package_name(name))
    }

    /// Gets a record status.
    pub async fn get_record_status(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<RecordStatus, CoreServiceError> {
        Ok(self.store.get_record_status(log_id, record_id).await?)
    }

    /// Gets a package log entry.
    pub async fn get_package_log_entry(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<PackageLogEntry, CoreServiceError> {
        Ok(self.store.get_package_log_entry(log_id, record_id).await?)
    }

    /// Gets the latest checkpoint.
    pub async fn get_latest_checkpoint(
        &self,
    ) -> Result<SerdeEnvelope<MapCheckpoint>, CoreServiceError> {
        Ok(self.store.get_latest_checkpoint().await?)
    }

    /// Fetches all operator records up until the given registry root.
    pub async fn fetch_operator_records(
        &self,
        root: &DynHash,
        since: Option<&RecordId>,
    ) -> Result<Vec<ProtoEnvelope<operator::OperatorRecord>>, CoreServiceError> {
        let log_id = LogId::operator_log::<Sha256>();
        Ok(self
            .store
            .get_operator_records(&log_id, root, since)
            .await?)
    }

    /// Fetches all package records up until the given registry root.
    pub async fn fetch_package_records(
        &self,
        name: &str,
        root: &DynHash,
        since: Option<&RecordId>,
    ) -> Result<Vec<ProtoEnvelope<package::PackageRecord>>, CoreServiceError> {
        let log_id = LogId::package_log::<Sha256>(name);
        self.store
            .get_package_records(&log_id, root, since)
            .await
            .map_err(|e| CoreServiceError::from(e).with_package_name(name))
    }
}
