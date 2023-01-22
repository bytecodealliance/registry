use std::{sync::Arc, collections::HashMap};

use anyhow::Error;
use forrest::map::ProofBundle;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use warg_crypto::hash::{Sha256, Hash};
use warg_protocol::registry::{LogId, MapLeaf};

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
    pub fn new(init: MapLeaf) -> Self {
        let map = VerifiableMap::default();
        let map = map.insert(LogId::operator_log::<Sha256>(), init);
        let mut map_index = HashMap::default();
        map_index.insert(map.root().clone(), map);
        Self { map_index }
    }

    pub fn inclusion(&self, root: Hash<Sha256>, log_ids: Vec<LogId>) -> Result<ProofBundle<Sha256, MapLeaf>, Error> {
        let map = self.map_index.get(&root).ok_or(Error::msg("Unknown map root"))?;

        let mut proofs = Vec::new();
        for log_id in log_ids {
            let proof = map.prove(&log_id).ok_or(Error::msg("Unable to prove"))?;
            proofs.push(proof);
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
