use super::DataServiceError;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{mpsc::Receiver, RwLock},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use warg_crypto::hash::{Hash, Sha256};
use warg_protocol::registry::{LogLeaf, LogId};
use warg_transparency::log::{LogBuilder, LogData as _, LogProofBundle, Node, VecLog};

pub struct Input {
    pub token: CancellationToken,
    pub data: LogData,
    pub log_rx: Receiver<LogLeaf>,
}

pub struct Output {
    pub data: Arc<RwLock<LogData>>,
    pub handle: JoinHandle<()>,
}

#[derive(Default)]
pub struct LogData {
    log: VecLog<Sha256, LogLeaf>,
    leaf_index: HashMap<LogLeaf, Node>,
    root_index: HashMap<Hash<Sha256>, usize>,
}

impl LogData {
    /// Push a new leaf into the log.
    pub fn push(&mut self, leaf: LogLeaf) {
        let node = self.log.push(&leaf);
        let checkpoint = self.log.checkpoint();
        self.root_index
            .insert(checkpoint.root(), checkpoint.length());
        self.leaf_index.insert(leaf, node);
    }

    /// Generate a proof bundle for the consistency of the log across two times
    pub fn consistency(
        &self,
        old_root: &Hash<Sha256>,
        new_root: &Hash<Sha256>,
    ) -> Result<LogProofBundle<Sha256, LogLeaf>, DataServiceError> {
        let old_len = self
            .root_index
            .get(old_root)
            .ok_or_else(|| DataServiceError::RootNotFound(old_root.clone()))?;
        let new_len = self
            .root_index
            .get(new_root)
            .ok_or_else(|| DataServiceError::RootNotFound(new_root.clone()))?;

        let proof = self.log.prove_consistency(*old_len, *new_len);
        let bundle = LogProofBundle::bundle(vec![proof], vec![], &self.log)
            .map_err(DataServiceError::BundleFailure)?;
        Ok(bundle)
    }

    /// Generate a proof bundle for a group of inclusion proofs
    pub fn inclusion(
        &self,
        root: &Hash<Sha256>,
        leaves: &[LogLeaf],
        exclusions: &[LogId]
    ) -> Result<LogProofBundle<Sha256, LogLeaf>, DataServiceError> {
        let log_length = *self
            .root_index
            .get(root)
            .ok_or_else(|| DataServiceError::RootNotFound(root.clone()))?;
        let mut proofs = Vec::new();
        for leaf in leaves {
            let node = *self
                .leaf_index
                .get(leaf)
                .ok_or_else(|| DataServiceError::LeafNotFound(leaf.clone()))?;
            let proof = self.log.prove_inclusion(node, log_length);
            proofs.push(proof);
        }
        let bundle = LogProofBundle::bundle(vec![], proofs, &self.log)
            .map_err(DataServiceError::BundleFailure)?;
        Ok(bundle)
    }
}

pub fn spawn(input: Input) -> Output {
    let Input {
        token,
        data,
        mut log_rx,
    } = input;
    let data = Arc::new(RwLock::new(data));
    let processor_data = data.clone();

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                leaf = log_rx.recv() => {
                    if let Some(leaf) = leaf {
                        let mut data = processor_data.as_ref().write().await;
                        data.push(leaf);
                    } else {
                        break;
                    }
                }
                _ = token.cancelled() => break,
            }
        }
    });

    Output { data, handle }
}
