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
    registry::{
        Checkpoint, LogId, LogLeaf, MapLeaf, RecordId, RegistryIndex, RegistryLen,
        TimestampedCheckpoint,
    },
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
        from_log_length: RegistryLen,
        to_log_length: RegistryLen,
    ) -> Result<LogProofBundle<Digest, LogLeaf>, CoreServiceError> {
        let state = self.inner.state.read().await;

        let proof = state
            .log
            .prove_consistency(from_log_length as usize, to_log_length as usize);
        LogProofBundle::bundle(vec![proof], vec![], &state.log)
            .map_err(CoreServiceError::BundleFailure)
    }

    /// Constructs log inclusion proofs for the given entries at the given log tree root.
    pub async fn log_inclusion_proofs(
        &self,
        log_length: RegistryLen,
        entries: &[RegistryIndex],
    ) -> Result<LogProofBundle<Digest, LogLeaf>, CoreServiceError> {
        let state = self.inner.state.read().await;

        let proofs = entries
            .iter()
            .map(|&index| {
                let node = if index < state.leaf_index.len() as RegistryIndex {
                    state.leaf_index[index as usize]
                } else {
                    return Err(CoreServiceError::LeafNotFound(index));
                };
                Ok(state.log.prove_inclusion(node, log_length as usize))
            })
            .collect::<Result<Vec<_>, CoreServiceError>>()?;

        LogProofBundle::bundle(vec![], proofs, &state.log).map_err(CoreServiceError::BundleFailure)
    }

    /// Constructs map inclusion proofs for the given entries at the given map tree root.
    pub async fn map_inclusion_proofs(
        &self,
        log_length: RegistryLen,
        entries: &[RegistryIndex],
    ) -> Result<MapProofBundle<Digest, LogId, MapLeaf>, CoreServiceError> {
        let state = self.inner.state.read().await;

        let (map_root, map) = state
            .map_index
            .get(&log_length)
            .ok_or_else(|| CoreServiceError::CheckpointNotFound(log_length))?;

        let indexes = self
            .inner
            .store
            .get_log_leafs_with_registry_index(entries)
            .await
            .map_err(CoreServiceError::DataStore)?;

        let proofs = indexes
            .iter()
            .map(|log_leaf| {
                let LogLeaf { log_id, record_id } = log_leaf;

                let proof = map
                    .prove(log_id.clone())
                    .ok_or_else(|| CoreServiceError::PackageNotIncluded(log_id.clone()))?;

                let map_leaf = MapLeaf {
                    record_id: record_id.clone(),
                };
                let found_root = proof.evaluate(log_id, &map_leaf);
                if &found_root != map_root {
                    return Err(CoreServiceError::IncorrectProof {
                        root: map_root.into(),
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
        let mut checkpoints_by_len: HashMap<RegistryLen, Checkpoint> = Default::default();
        while let Some(checkpoint) = checkpoints.next().await {
            let checkpoint = checkpoint?.checkpoint;
            checkpoints_by_len.insert(checkpoint.log_length, checkpoint);
        }

        let state = self.state.get_mut();
        while let Some(entry) = published.next().await {
            state.push_entry(entry?);
            if let Some(stored_checkpoint) =
                checkpoints_by_len.get(&(state.log.length() as RegistryLen))
            {
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
        let record_id = RecordId::operator_record::<Digest>(signed_init_record.content_bytes());

        // Store init record
        self.store
            .store_operator_record(&log_id, &record_id, &signed_init_record)
            .await?;
        self.store
            .commit_operator_record(&log_id, &record_id, 0)
            .await?;

        // Update state with init record
        state.push_entry(LogLeaf { log_id, record_id });

        // "zero" checkpoint to be updated
        let mut checkpoint = Checkpoint {
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
            .into_contents()
            .checkpoint;

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
        let registry_index = state.log.length() as RegistryIndex;
        let commit_res = self
            .store
            .commit_package_record(log_id, record_id, registry_index)
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
    async fn update_checkpoint(&self, checkpoint: &mut Checkpoint) {
        {
            // Recalculate the checkpoint if necessary
            let mut state = self.state.write().await;
            if state.log.length() as RegistryLen != checkpoint.log_length {
                *checkpoint = state.checkpoint();
                tracing::debug!("Updating to checkpoint {checkpoint:?}");
            }
        }

        if let Err(err) = self.sign_and_store_checkpoint(checkpoint.clone()).await {
            tracing::error!("Error storing checkpoint {checkpoint:?}: {err:?}");
        }
    }

    async fn sign_and_store_checkpoint(&self, checkpoint: Checkpoint) -> anyhow::Result<()> {
        let checkpoint_id = Hash::<Digest>::of(&checkpoint).into();
        let timestamped = TimestampedCheckpoint::now(checkpoint.clone())?;
        let signed = SerdeEnvelope::signed_contents(&self.operator_key, timestamped)?;
        self.store.store_checkpoint(&checkpoint_id, signed).await?;
        Ok(())
    }
}

type VerifiableMap<Digest> = Map<Digest, LogId, MapLeaf>;

#[derive(Default)]
struct State<Digest: SupportedDigest> {
    // The verifiable log of all package log entries
    log: VecLog<Digest, LogLeaf>,
    // Index log tree nodes by registry log index of the record
    leaf_index: Vec<Node>,

    // The verifiable map of package logs' latest entries (log_id -> record_id)
    map: VerifiableMap<Digest>,
    // Index verifiable map snapshots by log length (at checkpoints only)
    map_index: HashMap<RegistryLen, (Hash<Digest>, VerifiableMap<Digest>)>,
}

impl<Digest: SupportedDigest> State<Digest> {
    fn push_entry(&mut self, log_leaf: LogLeaf) {
        let node = self.log.push(&log_leaf);
        self.leaf_index.push(node);

        let LogLeaf { log_id, record_id } = log_leaf;
        self.map = self.map.insert(log_id, MapLeaf { record_id });
    }

    fn checkpoint(&mut self) -> Checkpoint {
        let log_checkpoint = self.log.checkpoint();
        let map_root = self.map.root();
        let log_length = log_checkpoint.length() as RegistryLen;

        // Update map snapshot
        if log_length > 0 {
            self.map_index
                .insert(log_length, (map_root.clone(), self.map.clone()));
        }

        Checkpoint {
            log_length,
            log_root: log_checkpoint.root().into(),
            map_root: map_root.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum CoreServiceError {
    #[error("checkpoint at log length `{0}` was not found")]
    CheckpointNotFound(RegistryLen),
    #[error("log leaf `{0}` was not found")]
    LeafNotFound(RegistryIndex),
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
