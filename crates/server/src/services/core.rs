use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use futures::{pin_mut, StreamExt};
use thiserror::Error;
use tokio::{
    sync::{mpsc, RwLock},
    task::JoinHandle,
    time::MissedTickBehavior,
};
use warg_crypto::{
    hash::{AnyHash, Hash, Sha256, SupportedDigest},
    signing::PrivateKey,
};
use warg_protocol::{
    operator,
    registry::{LogId, LogLeaf, MapCheckpoint, MapLeaf, RecordId},
    ProtoEnvelope, SerdeEnvelope,
};
use warg_transparency::{
    log::{LogBuilder, LogData, LogProofBundle, Node, VecLog},
    map::{Map, MapProofBundle},
};

use crate::datastore::{DataStore, DataStoreError};

#[derive(Clone)]
pub struct CoreService<Digest: SupportedDigest = Sha256> {
    inner: Arc<Inner<Digest>>,

    // Channel sender used by `submit_package_record` to serialize submissions.
    submit_entry_tx: mpsc::Sender<LogLeaf>,
}

impl<Digest: SupportedDigest> CoreService<Digest> {
    /// Starts the `CoreService`, returning a `clone`able handle to the
    /// service and a [`JoinHandle`] which should be awaited after dropping all
    /// copies of the service handle to allow for graceful shutdown.
    pub async fn start(
        operator_key: PrivateKey,
        store: Box<dyn DataStore>,
        checkpoint_interval: Duration,
    ) -> Result<(Self, JoinHandle<()>), CoreServiceError> {
        // Build service
        let mut inner = Inner {
            operator_key,
            store,
            state: Default::default(),
        };
        inner.initialize().await?;

        // Spawn state update task
        let inner = Arc::new(inner);
        let (submit_entry_tx, submit_entry_rx) = tokio::sync::mpsc::channel(4);
        let handle = tokio::spawn(
            inner
                .clone()
                .process_state_updates(submit_entry_rx, checkpoint_interval),
        );

        let svc = Self {
            inner,
            submit_entry_tx,
        };
        Ok((svc, handle))
    }

    /// Constructs a log consistency proof between the given log tree roots.
    pub async fn log_consistency_proof(
        &self,
        old_root: &Hash<Digest>,
        new_root: &Hash<Digest>,
    ) -> Result<LogProofBundle<Digest, LogLeaf>, CoreServiceError> {
        let state = self.inner.state.read().await;

        let old_length = state.get_log_len_at(old_root)?;
        let new_length = state.get_log_len_at(new_root)?;

        let proof = state.log.prove_consistency(old_length, new_length);
        LogProofBundle::bundle(vec![proof], vec![], &state.log)
            .map_err(CoreServiceError::BundleFailure)
    }

    /// Constructs log inclusion proofs for the given entries at the given log tree root.
    pub async fn log_inclusion_proofs(
        &self,
        root: &Hash<Digest>,
        entries: &[LogLeaf],
    ) -> Result<LogProofBundle<Digest, LogLeaf>, CoreServiceError> {
        let state = self.inner.state.read().await;

        let log_length = state.get_log_len_at(root)?;

        let proofs = entries
            .iter()
            .map(|entry| {
                let node = state
                    .leaf_index
                    .get(entry)
                    .ok_or_else(|| CoreServiceError::LeafNotFound(entry.clone()))?;
                Ok(state.log.prove_inclusion(*node, log_length))
            })
            .collect::<Result<Vec<_>, CoreServiceError>>()?;

        LogProofBundle::bundle(vec![], proofs, &state.log).map_err(CoreServiceError::BundleFailure)
    }

    /// Constructs map inclusion proofs for the given entries at the given map tree root.
    pub async fn map_inclusion_proofs(
        &self,
        root: &Hash<Digest>,
        entries: &[LogLeaf],
    ) -> Result<MapProofBundle<Digest, LogId, MapLeaf>, CoreServiceError> {
        let state = self.inner.state.read().await;

        let map = state
            .map_index
            .get(root)
            .ok_or_else(|| CoreServiceError::RootNotFound(root.into()))?;

        let proofs = entries
            .iter()
            .map(|entry| {
                let LogLeaf { log_id, record_id } = entry;

                let proof = map
                    .prove(log_id.clone())
                    .ok_or_else(|| CoreServiceError::PackageNotIncluded(log_id.clone()))?;

                let map_leaf = MapLeaf {
                    record_id: record_id.clone(),
                };
                let found_root = proof.evaluate(log_id, &map_leaf);
                if &found_root != root {
                    return Err(CoreServiceError::IncorrectProof {
                        root: root.into(),
                        found: found_root.into(),
                    });
                }

                Ok(proof)
            })
            .collect::<Result<Vec<_>, CoreServiceError>>()?;

        Ok(MapProofBundle::bundle(proofs))
    }

    /// Gets the data store associated with the transparency service.
    pub fn store(&self) -> &dyn DataStore {
        self.inner.store.as_ref()
    }

    /// Submits a package record to be processed.
    pub async fn submit_package_record(&self, log_id: LogId, record_id: RecordId) {
        self.submit_entry_tx
            .send(LogLeaf { log_id, record_id })
            .await
            .unwrap()
    }
}

struct Inner<Digest: SupportedDigest> {
    // Operator signing key
    operator_key: PrivateKey,

    // DataStore persists transparency state.
    store: Box<dyn DataStore>,

    // In-memory transparency state.
    state: RwLock<State<Digest>>,
}

impl<Digest: SupportedDigest> Inner<Digest> {
    // Load state from DataStore or initialize empty state, returning any
    // entries that are not yet part of a checkpoint.
    async fn initialize(&mut self) -> Result<(), CoreServiceError> {
        tracing::debug!("Initializing CoreService");

        let published = self.store.get_all_validated_records().await?.peekable();
        pin_mut!(published);

        // If there are no published records, initialize a new state
        if published.as_mut().peek().await.is_none() {
            tracing::debug!("No existing records; initializing new state");
            return self.initialize_new().await;
        }

        // Reconstruct internal state from previously-stored data
        let mut checkpoints = self.store.get_all_checkpoints().await?;
        let mut checkpoints_by_len: HashMap<usize, MapCheckpoint> = Default::default();
        while let Some(checkpoint) = checkpoints.next().await {
            let checkpoint = checkpoint?;
            checkpoints_by_len.insert(checkpoint.log_length as usize, checkpoint);
        }

        let state = self.state.get_mut();
        while let Some(entry) = published.next().await {
            state.push_entry(entry?);
            if let Some(stored_checkpoint) = checkpoints_by_len.get(&state.log.length()) {
                // Validate stored checkpoint (and update internal state as a side-effect)
                let computed_checkpoint = state.checkpoint();
                assert!(stored_checkpoint == &computed_checkpoint);
            }
        }

        Ok(())
    }

    async fn initialize_new(&mut self) -> Result<(), CoreServiceError> {
        let state = self.state.get_mut();

        // Construct operator init record
        let init_record = operator::OperatorRecord {
            prev: None,
            version: 0,
            timestamp: SystemTime::now(),
            entries: vec![operator::OperatorEntry::Init {
                hash_algorithm: Digest::ALGORITHM,
                key: self.operator_key.public_key(),
            }],
        };
        let signed_init_record =
            ProtoEnvelope::signed_contents(&self.operator_key, init_record).unwrap();
        let log_id = LogId::operator_log::<Digest>();
        let record_id = RecordId::operator_record::<Digest>(&signed_init_record);

        // Store init record
        self.store
            .store_operator_record(&log_id, &record_id, &signed_init_record)
            .await?;
        self.store
            .commit_operator_record(&log_id, &record_id, 0)
            .await?;

        // Update state with init record
        let entry = LogLeaf { log_id, record_id };
        state.push_entry(entry.clone());

        // "zero" checkpoint to be updated
        let mut checkpoint = MapCheckpoint {
            log_root: Hash::<Digest>::default().into(),
            log_length: 0,
            map_root: Hash::<Digest>::default().into(),
        };
        self.update_checkpoint(&mut checkpoint).await;

        Ok(())
    }

    // Runs the service's state update loop.
    async fn process_state_updates(
        self: Arc<Self>,
        mut submit_entry_rx: mpsc::Receiver<LogLeaf>,
        checkpoint_interval: Duration,
    ) {
        let mut checkpoint = self
            .store
            .get_latest_checkpoint()
            .await
            .unwrap()
            .into_contents();
        let mut checkpoint_interval = tokio::time::interval(checkpoint_interval);
        checkpoint_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                entry = submit_entry_rx.recv() => match entry {
                    Some(entry) => self.process_package_entry(&entry).await,
                    None => break, // Channel closed
                },
                _ = checkpoint_interval.tick() => self.update_checkpoint(&mut checkpoint).await,
            }
        }
    }

    // Processes a submitted package entry
    async fn process_package_entry(&self, entry: &LogLeaf) {
        tracing::debug!("Processing entry {entry:?}");

        let mut state = self.state.write().await;
        let LogLeaf { log_id, record_id } = entry;

        // Validate and commit the package entry to the store
        let registry_log_index = state.log.length().try_into().unwrap();
        let commit_res = self
            .store
            .commit_package_record(log_id, record_id, registry_log_index)
            .await;

        if let Err(err) = commit_res {
            match err {
                DataStoreError::Rejection(_)
                | DataStoreError::OperatorValidationFailed(_)
                | DataStoreError::PackageValidationFailed(_) => {
                    // The record failed to validate and was rejected; do not include it in the next checkpoint
                    tracing::debug!("record `{record_id}` rejected: {err:?}");
                }
                e => {
                    // TODO: this should be made more robust with a proper reliable message
                    // queue with retry logic
                    tracing::error!("failed to validate package record `{record_id}`: {e}");
                }
            }
            return;
        }

        state.push_entry(entry.clone());
    }

    // Store a checkpoint including the given new entries
    async fn update_checkpoint(&self, current_checkpoint: &mut MapCheckpoint) {
        let mut state = self.state.write().await;
        if state.log.length() == (current_checkpoint.log_length as usize) {
            return;
        }
        let next_checkpoint = state.checkpoint();
        tracing::debug!("Updating to checkpoint {next_checkpoint:?}");
        let signed_checkpoint =
            SerdeEnvelope::signed_contents(&self.operator_key, next_checkpoint.clone()).unwrap();
        let checkpoint_id = Hash::<Digest>::of(signed_checkpoint.as_ref()).into();
        self.store
            .store_checkpoint(&checkpoint_id, signed_checkpoint)
            .await
            .unwrap();
        *current_checkpoint = next_checkpoint;
    }
}

type VerifiableMap<Digest> = Map<Digest, LogId, MapLeaf>;

#[derive(Default)]
struct State<Digest: SupportedDigest> {
    // The verifiable log of all package log entries
    log: VecLog<Digest, LogLeaf>,
    // Index log tree nodes by entry
    leaf_index: HashMap<LogLeaf, Node>,
    // Index log size by log tree root
    root_index: HashMap<Hash<Digest>, usize>,

    // The verifiable map of package logs' latest entries (log_id -> record_id)
    map: VerifiableMap<Digest>,
    // Index verifiable map snapshots by root (at checkpoints only)
    map_index: HashMap<Hash<Digest>, VerifiableMap<Digest>>,
}

impl<Digest: SupportedDigest> State<Digest> {
    fn push_entry(&mut self, entry: LogLeaf) {
        let node = self.log.push(&entry);
        self.leaf_index.insert(entry.clone(), node);

        let log_checkpoint = self.log.checkpoint();
        self.root_index
            .insert(log_checkpoint.root(), log_checkpoint.length());

        self.map = self.map.insert(
            entry.log_id.clone(),
            MapLeaf {
                record_id: entry.record_id.clone(),
            },
        );
    }

    fn checkpoint(&mut self) -> MapCheckpoint {
        let log_checkpoint = self.log.checkpoint();
        let map_root = self.map.root();

        // Update map snapshot
        if log_checkpoint.length() > 0 {
            self.map_index.insert(map_root.clone(), self.map.clone());
        }

        MapCheckpoint {
            log_length: log_checkpoint.length().try_into().unwrap(),
            log_root: log_checkpoint.root().into(),
            map_root: map_root.into(),
        }
    }

    fn get_log_len_at(&self, log_root: &Hash<Digest>) -> Result<usize, CoreServiceError> {
        self.root_index
            .get(log_root)
            .cloned()
            .ok_or_else(|| CoreServiceError::RootNotFound(log_root.into()))
    }
}

#[derive(Debug, Error)]
pub enum CoreServiceError {
    #[error("root `{0}` was not found")]
    RootNotFound(AnyHash),
    #[error("log leaf `{}:{}` was not found", .0.log_id, .0.record_id)]
    LeafNotFound(LogLeaf),
    #[error("failed to bundle proofs: `{0}`")]
    BundleFailure(anyhow::Error),
    #[error("failed to prove inclusion of package `{0}`")]
    PackageNotIncluded(LogId),
    #[error("failed to prove inclusion: found root `{found}` but was given root `{root}`")]
    IncorrectProof { root: AnyHash, found: AnyHash },
    #[error("data store error: {0}")]
    DataStore(#[from] DataStoreError),
    #[error("initialization failed: {0}")]
    InitializationFailure(String),
}
