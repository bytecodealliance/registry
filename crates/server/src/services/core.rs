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

use crate::datastore::{DataStore, DataStoreError, InitialLeaf};

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
        let pending_entries = inner.initialize().await?;

        // Spawn state update task
        let inner = Arc::new(inner);
        let (submit_entry_tx, submit_entry_rx) = tokio::sync::mpsc::channel(4);
        let handle = tokio::spawn(inner.clone().process_state_updates(
            pending_entries,
            submit_entry_rx,
            checkpoint_interval,
        ));

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
    async fn initialize(&mut self) -> Result<Vec<LogLeaf>, CoreServiceError> {
        let initial = self.store.get_initial_leaves().await?.peekable();
        pin_mut!(initial);

        // If there are no stored log entries, initialize a new state
        if initial.as_mut().peek().await.is_none() {
            self.initialize_new().await?;
            return Ok(vec![]);
        }

        // Reconstruct internal state from previously-stored log entires
        let state = self.state.get_mut();
        let mut pending_entries = vec![];
        let mut last_checkpoint = Hash::<Digest>::of(()).into();

        while let Some(res) = initial.next().await {
            let InitialLeaf { leaf, checkpoint } = res?;

            // Push the entry itself
            state.push_entry(leaf.clone());

            // Update per-checkpoint state and track any entries that aren't
            // yet part of a checkpoint
            if let Some(checkpoint) = checkpoint {
                if last_checkpoint != checkpoint {
                    state.checkpoint(); // for the side-effect of updating map_index
                    last_checkpoint = checkpoint;
                }
                pending_entries.clear();
            } else {
                pending_entries.push(leaf);
            }
        }

        Ok(pending_entries)
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
            .validate_operator_record(&log_id, &record_id)
            .await?;

        // Update state with init record
        let entry = LogLeaf { log_id, record_id };
        state.push_entry(entry.clone());

        self.update_checkpoint(vec![entry]).await;

        Ok(())
    }

    // Runs the service's state update loop.
    async fn process_state_updates(
        self: Arc<Self>,
        mut pending_entries: Vec<LogLeaf>,
        mut submit_entry_rx: mpsc::Receiver<LogLeaf>,
        checkpoint_interval: Duration,
    ) {
        let mut checkpoint_interval = tokio::time::interval(checkpoint_interval);
        checkpoint_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                entry = submit_entry_rx.recv() => match entry {
                    Some(entry) => {
                        if self.process_package_entry(&entry).await.is_ok() {
                            pending_entries.push(entry);
                        }
                    }
                    None => break, // Channel closed
                },
                _ = checkpoint_interval.tick() => {
                    let new_entries = std::mem::take(&mut pending_entries);
                    self.update_checkpoint(new_entries).await;
                },
            }
        }
    }

    // Processes a submitted package entry
    async fn process_package_entry(&self, entry: &LogLeaf) -> Result<(), ()> {
        let LogLeaf { log_id, record_id } = entry;

        // Validate and commit the package entry to the store
        self.store
            .validate_package_record(log_id, record_id)
            .await
            .map_err(|err| match err {
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
            })?;

        let mut state = self.state.write().await;
        state.push_entry(entry.clone());
        Ok(())
    }

    // Store a checkpoint including the given new entries
    async fn update_checkpoint(&self, new_entries: Vec<LogLeaf>) {
        if new_entries.is_empty() {
            return;
        }
        let checkpoint = self.state.write().await.checkpoint();
        let signed_checkpoint =
            SerdeEnvelope::signed_contents(&self.operator_key, checkpoint.clone()).unwrap();
        let checkpoint_id = Hash::<Digest>::of(signed_checkpoint.as_ref()).into();
        self.store
            .store_checkpoint(&checkpoint_id, signed_checkpoint, &new_entries)
            .await
            .unwrap();
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
    IniitializationFailure(String),
}
