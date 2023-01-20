use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Error;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use forrest::log::{LogBuilder, VecLog, ProofBundle, ConsistencyProof, InclusionProof, Node};
use warg_crypto::hash::{Sha256, Hash};
use warg_protocol::registry::LogLeaf;
use warg_protocol::Encode;

pub type ProofLog = VecLog<Sha256>;

pub struct Input {
    pub data: LogData,
    pub log_rx: Receiver<LogLeaf>,
}

pub struct Output {
    pub data: Arc<RwLock<LogData>>,
    pub handle: JoinHandle<()>,
}

#[derive(Default)]
pub struct LogData {
    log: ProofLog,
    leaf_index: HashMap<LogLeaf, Node>,
    root_index: HashMap<Hash<Sha256>, usize>
}

impl LogData {
    /// Generate a proof bundle for the consistency of the log across two times
    pub fn consistency(&self, old_root: Hash<Sha256>, new_root: Hash<Sha256>) -> Result<ProofBundle<Sha256>, Error> {
        let old_len = self.root_index.get(&old_root).ok_or(Error::msg("Old root not found"))?;
        let new_len = self.root_index.get(&new_root).ok_or(Error::msg("New root not found"))?;

        let proof = ConsistencyProof {
            old_length: *old_len,
            new_length: *new_len,
        };
        let bundle = ProofBundle::bundle(vec![proof], vec![], &self.log)?;
        Ok(bundle)
    }

    /// Generate a proof bundle for a group of inclusion proofs
    pub fn inclusion(&self, root: Hash<Sha256>, leaves: Vec<LogLeaf>) -> Result<ProofBundle<Sha256>, Error> {
        let log_length = self.root_index.get(&root).ok_or(Error::msg("Root not found"))?.clone();
        let mut proofs = Vec::new();
        for leaf in leaves {
            let node = self.leaf_index.get(&leaf).ok_or(Error::msg("Leaf not found"))?.clone();
            let proof = InclusionProof {
                log_length,
                leaf: node,
            };
            proofs.push(proof);
        }
        let bundle = ProofBundle::bundle(vec![], proofs, &self.log)?;
        Ok(bundle)
    }
}

pub fn process(input: Input) -> Output {
    let Input { data, mut log_rx } = input;
    let data = Arc::new(RwLock::new(data));
    let processor_data = data.clone();

    let handle = tokio::spawn(async move {
        let data = processor_data;

        while let Some(leaf) = log_rx.recv().await {
            let mut data = data.as_ref().blocking_write();
            let node = data.log.push(leaf.encode());

            let checkpoint = data.log.checkpoint();
            data.root_index.insert(checkpoint.root(), checkpoint.length());
            data.leaf_index.insert(leaf, node);

            drop(data);
        }
    });

    Output { data, handle }
}
