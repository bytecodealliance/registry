use forrest::map::Map;
use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;

use warg_protocol::Encode;
use warg_crypto::hash::Sha256;
use warg_protocol::registry::{MapLeaf, MapCheckpoint, LogId, LogLeaf};

use super::log;

pub type VerifiableMap = Map<Sha256, LogId, Vec<u8>>;

struct Input {
    map: VerifiableMap,
    map_rx: Receiver<log::Summary>
}

struct Output {
    summary_rx: Receiver<Summary>,
    map_data_rx: Receiver<VerifiableMap>,
    handle: JoinHandle<VerifiableMap>
}

#[derive(Debug)]
pub struct Summary {
    leaves: Vec<LogLeaf>,
    checkpoint: MapCheckpoint
}

async fn process(input: Input) -> Output {
    let (summary_tx, summary_rx) = mpsc::channel::<Summary>(4);
    let (map_data_tx, map_data_rx) = mpsc::channel::<VerifiableMap>(4);

    let handle = tokio::spawn(async move {
        let Input { mut map, mut map_rx } = input;
        let mut leaves = Vec::new();

        while let Some(message) = map_rx.recv().await {
            let leaf = message.leaf;
            map = map.insert(leaf.log_id.clone(), MapLeaf { record_id: leaf.record_id.clone() }.encode());
            leaves.push(leaf);

            let checkpoint = MapCheckpoint {
                log_root: message.log_root,
                log_length: message.log_length,
                map_root: map.root().clone().into(),
            };

            map_data_tx.send(map.clone()).await.unwrap();
            // TODO: only send sign message every few seconds
            summary_tx.send(Summary { leaves, checkpoint }).await.unwrap();
            leaves = Vec::new();
        }

        map
    });

    Output {
        summary_rx,
        map_data_rx,
        handle
    }
}