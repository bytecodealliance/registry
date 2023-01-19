use std::time::Duration;

use forrest::map::Map;
use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;

use tokio::time;
use warg_crypto::hash::Sha256;
use warg_protocol::registry::{LogId, LogLeaf, MapCheckpoint, MapLeaf};
use warg_protocol::Encode;

use super::log;

pub type VerifiableMap = Map<Sha256, LogId, Vec<u8>>;

pub struct Input {
    pub map: VerifiableMap,
    pub map_rx: Receiver<log::Summary>,
}

pub struct Output {
    pub summary_rx: Receiver<Summary>,
    pub map_data_rx: Receiver<VerifiableMap>,
    pub handle: JoinHandle<VerifiableMap>,
}

#[derive(Debug)]
pub struct Summary {
    pub leaves: Vec<LogLeaf>,
    pub checkpoint: MapCheckpoint,
}

pub async fn process(input: Input) -> Output {
    let (summary_tx, summary_rx) = mpsc::channel::<Summary>(4);
    let (map_data_tx, map_data_rx) = mpsc::channel::<VerifiableMap>(4);

    let mut interval = time::interval(Duration::from_secs(5));

    let handle = tokio::spawn(async move {
        let Input {
            mut map,
            mut map_rx,
        } = input;
        let mut current = None;
        let mut leaves = Vec::new();

        loop {
            tokio::select! {
                message = map_rx.recv() => {
                    if let Some(message) = message {
                        let leaf = message.leaf;
                        map = map.insert(leaf.log_id.clone(), MapLeaf { record_id: leaf.record_id.clone() }.encode());
                        leaves.push(leaf);

                        current = Some(MapCheckpoint {
                            log_root: message.log_root,
                            log_length: message.log_length,
                            map_root: map.root().clone().into(),
                        });
                    } else {
                        break;
                    }
                },
                _ = interval.tick() => {
                    if let Some(checkpoint) = current {
                        map_data_tx.send(map.clone()).await.unwrap();
                        summary_tx.send(Summary { leaves, checkpoint }).await.unwrap();
                        leaves = Vec::new();
                        current = None;
                    }
                }
            }
        }

        if let Some(checkpoint) = current {
            map_data_tx.send(map.clone()).await.unwrap();
            summary_tx
                .send(Summary { leaves, checkpoint })
                .await
                .unwrap();
        }

        map
    });

    Output {
        summary_rx,
        map_data_rx,
        handle,
    }
}
