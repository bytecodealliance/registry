use super::log;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;
use tokio::time;
use tokio_util::sync::CancellationToken;
use warg_crypto::hash::Sha256;
use warg_protocol::registry::{LogId, LogLeaf, MapCheckpoint, MapLeaf};
use warg_transparency::map::Map;

pub type VerifiableMap = Map<Sha256, LogId, MapLeaf>;

pub struct Input {
    pub token: CancellationToken,
    pub checkpoint_interval: Duration,
    pub map: VerifiableMap,
    pub leaves: Vec<LogLeaf>,
    pub log_summary_rx: Receiver<log::Summary>,
}

pub struct Output {
    pub map_summary_rx: Receiver<Summary>,
    pub map_rx: Receiver<VerifiableMap>,
    pub handle: JoinHandle<()>,
}

#[derive(Debug)]
pub struct Summary {
    pub leaves: Vec<LogLeaf>,
    pub checkpoint: MapCheckpoint,
}

pub fn spawn(input: Input) -> Output {
    let (map_summary_tx, map_summary_rx) = mpsc::channel::<Summary>(4);
    let (map_tx, map_rx) = mpsc::channel::<VerifiableMap>(4);
    let handle = tokio::spawn(async move {
        let Input {
            token,
            checkpoint_interval,
            mut map,
            mut leaves,
            mut log_summary_rx,
        } = input;
        let mut current = None;
        let mut interval = time::interval(checkpoint_interval);

        loop {
            tokio::select! {
                summary = log_summary_rx.recv() => {
                    if let Some(summary) = summary {
                        let leaf = summary.leaf;
                        map = map.insert(leaf.log_id.clone(), MapLeaf { record_id: leaf.record_id.clone() });
                        leaves.push(leaf);

                        current = Some(MapCheckpoint {
                            log_root: summary.log_root,
                            log_length: summary.log_length,
                            map_root: map.root().clone().into(),
                        });
                    } else {
                        break;
                    }
                },
                _ = interval.tick() => {
                    if let Some(checkpoint) = current {
                        tracing::debug!("creating checkpoint with {len} new leaves", len = leaves.len());
                        map_tx.send(map.clone()).await.unwrap();
                        map_summary_tx.send(Summary { leaves, checkpoint }).await.unwrap();
                        leaves = Vec::new();
                        current = None;
                    }
                }
                _ = token.cancelled() => {
                    break;
                }
            }
        }

        if let Some(checkpoint) = current {
            map_tx.send(map.clone()).await.unwrap();
            map_summary_tx
                .send(Summary { leaves, checkpoint })
                .await
                .unwrap();
        }
    });

    Output {
        map_summary_rx,
        map_rx,
        handle,
    }
}
