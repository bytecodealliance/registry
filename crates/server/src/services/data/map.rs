use super::DataServiceError;
use crate::services::transparency::VerifiableMap;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{mpsc::Receiver, RwLock},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use warg_crypto::hash::{Hash, Sha256};
use warg_protocol::registry::{LogId, LogLeaf, MapLeaf};
use warg_transparency::map::MapProofBundle;

pub struct Input {
    pub token: CancellationToken,
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
    pub fn insert(&mut self, map: VerifiableMap) {
        self.map_index.insert(map.root().clone(), map);
    }

    pub fn inclusion(
        &self,
        root: &Hash<Sha256>,
        leaves: &[LogLeaf],
    ) -> Result<MapProofBundle<Sha256, LogId, MapLeaf>, DataServiceError> {
        let map = self
            .map_index
            .get(root)
            .ok_or_else(|| DataServiceError::RootNotFound(root.clone()))?;
        let mut proofs = Vec::new();
        for LogLeaf { log_id, record_id } in leaves {
            let proof = map
                .prove(log_id.clone())
                .ok_or_else(|| DataServiceError::PackageNotIncluded(log_id.clone()))?;
            let leaf = MapLeaf {
                record_id: record_id.clone(),
            };
            let found_root = proof.evaluate(log_id, &leaf);
            if found_root != *root {
                return Err(DataServiceError::IncorrectProof {
                    root: root.clone(),
                    found: found_root,
                });
            }
            proofs.push(proof);
        }

        Ok(MapProofBundle::bundle(proofs))
    }
}

pub fn spawn(input: Input) -> Output {
    let Input {
        token,
        data,
        mut map_rx,
    } = input;
    let data = Arc::new(RwLock::new(data));
    let processor_data = data.clone();

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                map = map_rx.recv() => {
                    if let Some(map) = map {
                        let mut data = processor_data.as_ref().write().await;
                        data.map_index.insert(map.root().clone(), map);
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
