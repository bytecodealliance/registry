use std::{sync::Arc, collections::HashMap};

use anyhow::{Error, Context};
use forrest::map::ProofBundle;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use warg_crypto::hash::{Sha256, Hash, DynHash};
use warg_protocol::registry::LogId;

use crate::services::transparency::VerifiableMap;

pub struct Input {
    pub data: MapData,
    pub map_rx: Receiver<VerifiableMap>,
}

pub struct Output {
    pub data: Arc<RwLock<MapData>>,
    pub handle: JoinHandle<()>,
}

#[derive(Default)]
pub struct MapData {
    map_index: HashMap<Hash<Sha256>, VerifiableMap>
}

impl MapData {
    pub fn inclusion(&self, root: Hash<Sha256>, log_ids: Vec<LogId>) -> Result<ProofBundle<Sha256, Vec<u8>>, Error> {
        let map = self.map_index.get(&root).ok_or(Error::msg("Unknown map root"))?;

        let mut proofs = Vec::new();
        for log_id in log_ids {
            let hash: DynHash = log_id.into();
            let hash: Hash<Sha256> = hash.try_into().with_context(|| Error::msg("Algorithm must be sha256"))?;
            let proof = map.prove(&hash).ok_or(Error::msg("Unable to prove"))?;
            proofs.push(proof.owned());
        }

        Ok(ProofBundle::bundle(proofs))
    }
}

pub fn process(input: Input) -> Output {
    let Input { data, mut map_rx } = input;
    let data = Arc::new(RwLock::new(data));
    let processor_data = data.clone();

    let handle = tokio::spawn(async move {
        let data = processor_data;

        while let Some(map) = map_rx.recv().await {
            let mut data = data.as_ref().blocking_write();
            data.map_index.insert(map.root().clone(), map);
            drop(data);
        }
    });

    Output { data, handle }
}
