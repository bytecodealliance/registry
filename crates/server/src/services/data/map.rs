use super::DataServiceError;
use crate::services::transparency::VerifiableMap;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{mpsc::Receiver, RwLock},
    task::JoinHandle,
};
use warg_crypto::hash::{Hash, Sha256};
use warg_protocol::registry::{LogId, LogLeaf, MapLeaf};
use warg_transparency::map::MapProofBundle;

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
    map_index: HashMap<Hash<Sha256>, VerifiableMap>,
}

impl MapData {
    pub fn new(init: MapLeaf) -> Self {
        let map = VerifiableMap::default();
        let map = map.insert(LogId::operator_log::<Sha256>(), init);
        let mut map_index = HashMap::default();
        map_index.insert(map.root().clone(), map);
        Self { map_index }
    }

    pub fn inclusion(
        &self,
        root: &Hash<Sha256>,
        leaves: &[LogLeaf],
    ) -> Result<MapProofBundle<Sha256, MapLeaf>, DataServiceError> {
        let map = self
            .map_index
            .get(root)
            .ok_or_else(|| DataServiceError::RootNotFound(root.clone()))?;

        let mut proofs = Vec::new();
        for LogLeaf { log_id, record_id } in leaves {
            let proof = map
                .prove(log_id)
                .ok_or_else(|| DataServiceError::ProofFailure(log_id.clone()))?;
            let leaf = MapLeaf {
                record_id: record_id.clone(),
            };
            let found_root = proof.evaluate(log_id, &leaf);
            if found_root != *root {
                return Err(DataServiceError::IncorrectProof {
                    root: root.clone(),
                    expected: found_root,
                });
            }
            proofs.push(proof);
        }

        Ok(MapProofBundle::bundle(proofs))
    }
}

pub fn process(input: Input) -> Output {
    let Input { data, mut map_rx } = input;
    let data = Arc::new(RwLock::new(data));
    let processor_data = data.clone();

    let handle = tokio::spawn(async move {
        let data = processor_data;

        while let Some(map) = map_rx.recv().await {
            let mut data = data.as_ref().write().await;
            data.map_index.insert(map.root().clone(), map);
            drop(data);
        }
    });

    Output { data, handle }
}
