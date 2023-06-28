use super::{
    data::log::LogData,
    transparency::{VerifiableLog, VerifiableMap},
    MapData,
};
use crate::{
    datastore::{DataStore, DataStoreError, InitialLeaf},
    services::{data, transparency},
};
use anyhow::Result;
use futures::StreamExt;
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{
        mpsc::{self, Sender},
        RwLock,
    },
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use warg_crypto::{
    hash::{AnyHash, Hash, HashAlgorithm, Sha256},
    signing::PrivateKey,
};
use warg_protocol::{
    operator,
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
struct SubmitPackageRecord {
    log_id: LogId,
    record_id: RecordId,
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
    submit_record_tx: Sender<SubmitPackageRecord>,
    store: Box<dyn DataStore>,
}

impl CoreService {
    /// Spawn the core service with the given operator signing key.
    pub async fn spawn(
        signing_key: PrivateKey,
        store: Box<dyn DataStore>,
        checkpoint_interval: Duration,
    ) -> Result<(Arc<Self>, StopHandle), DataStoreError> {
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

        let (submit_record_tx, mut submit_record_rx) = mpsc::channel(4);
        let core = Arc::new(Self {
            log: data.log_data,
            map: data.map_data,
            submit_record_tx,
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
                    message = submit_record_rx.recv() => match message {
                        Some(SubmitPackageRecord {
                            log_id, record_id
                        }) => {
                            // Validate the package record
                            match task_core.store.validate_package_record(&log_id, &record_id).await {
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
                                        tracing::error!("failed to validate package record `{record_id}`: {e}");
                                    }
                                }
                            }
                        }
                        None => break,
                    },
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
    ) -> Result<InitializationData, DataStoreError> {
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
    ) -> Result<InitializationData, DataStoreError> {
        let init = init_envelope(signing_key);
        let log_id = LogId::operator_log::<Sha256>();
        let record_id = RecordId::operator_record::<Sha256>(&init);

        store
            .store_operator_record(&log_id, &record_id, &init)
            .await?;

        // TODO: ensure the operator record passes all policy checks

        store.validate_operator_record(&log_id, &record_id).await?;

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
        let log_root: AnyHash = checkpoint.root().into();
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
    /// Gets the data store associated with the core service.
    pub fn store(&self) -> &dyn DataStore {
        self.store.as_ref()
    }

    /// Submits a package record to be processed.
    pub async fn submit_package_record(&self, log_id: LogId, record_id: RecordId) {
        self.submit_record_tx
            .send(SubmitPackageRecord { log_id, record_id })
            .await
            .unwrap();
    }
}
