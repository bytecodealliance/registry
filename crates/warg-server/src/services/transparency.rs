use forrest::map::Map;
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::task::JoinHandle;

use forrest::log::{VecLog, VerifiableLog};
use warg_protocol::Encode;
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::registry::{LogLeaf, MapLeaf, MapCheckpoint, LogId, RecordId};

pub struct TransparencyService {
    mailbox: Sender<LogLeaf>,
    handles: [JoinHandle<()>; 4],
}

#[derive(Debug)]
enum MapMessage {
    NewRoot {
        leaf: LogLeaf,
        log_root: DynHash,
        log_length: u32
    },
    /// It's time to generate a new top root
    Tick
}

#[derive(Debug)]
struct SignMessage {
    leaves: Vec<LogLeaf>,
    checkpoint: MapCheckpoint
}


impl TransparencyService {
    pub fn new() -> Self {
        let (mailbox, log_rx) = mpsc::channel::<LogLeaf>(4);
        let (map_tx, map_rx) = mpsc::channel::<MapMessage>(4);
        let (sign_tx, sign_rx) = mpsc::channel::<SignMessage>(4);
        // let (top_tx, top_rx) = mpsc::channel::<SignMessage>(4);

        let log_handle = tokio::spawn(async move {
            Self::log_process(log_rx, map_tx).await;
        });
        let map_handle = tokio::spawn(async move {
            Self::map_process(map_rx, sign_tx).await;
        });
        let sign_handle = tokio::spawn(async move {
            // Self::sign_process(sign_rx, cp_tx).await;
        });
        let checkpoint_handle = tokio::spawn(async move {
            // Self::top_process(cp_rx).await;
        });

        Self { mailbox, handles: [log_handle, map_handle, sign_handle, checkpoint_handle] }
    }

    async fn log_process(mut log_rx: Receiver<LogLeaf>, map_tx: Sender<MapMessage>) {
        let mut log: VecLog<Sha256> = VecLog::default();

        while let Some(leaf) = log_rx.recv().await {
            log.push(leaf.encode());
            let log_root: DynHash = log.root().into();
            let log_length = log.checkpoint().length() as u32;

            map_tx.send(MapMessage::NewRoot { leaf, log_root, log_length }).await.unwrap();
        }
    }

    async fn map_process(mut map_rx: Receiver<MapMessage>, sign_tx: Sender<SignMessage>) {
        let mut map: Map<Sha256, LogId, Vec<u8>> = Map::default();

        let mut root = None;
        let mut leaves = Vec::new();

        while let Some(message) = map_rx.recv().await {
            match message {
                MapMessage::NewRoot { leaf, log_root, log_length } => {
                    map = map.insert(leaf.log_id.clone(), MapLeaf { record_id: leaf.record_id.clone() }.encode());
                    leaves.push(leaf);

                    root = Some(MapCheckpoint {
                        log_root,
                        log_length,
                        map_root: map.root().clone().into(),
                    });
                },
                MapMessage::Tick => {
                    if let Some(checkpoint) = root {
                        sign_tx.send(SignMessage { leaves, checkpoint }).await.unwrap();
                        root = None;
                        leaves = Vec::new();
                    }
                },
            }

        }
    }

    async fn sign_process(mut sign_rx: Receiver<SignMessage>) {
        // start

        while let Some(info) = sign_rx.recv().await {
            // process
        }
    }
}

impl TransparencyService {
    pub async fn record(&self, log_id: LogId, record_id: RecordId) {
        self.mailbox.send(LogLeaf { log_id, record_id }).await.unwrap();
    }
}

