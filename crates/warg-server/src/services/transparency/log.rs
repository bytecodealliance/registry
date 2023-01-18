use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;

use forrest::log::{StackLog, LogBuilder};
use warg_protocol::Encode;
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::registry::LogLeaf;

pub struct Input {
    pub log: StackLog<Sha256>,
    pub log_rx: Receiver<LogLeaf>
}

pub struct Output {
    pub summary_rx: Receiver<Summary>,
    pub log_data_rx: Receiver<LogLeaf>,
    pub handle: JoinHandle<StackLog<Sha256>>
}

#[derive(Debug)]
pub struct Summary {
    pub leaf: LogLeaf,
    pub log_root: DynHash,
    pub log_length: u32
}

async fn process(input: Input) -> Output {
    let (summary_tx, summary_rx) = mpsc::channel::<Summary>(4);
    let (log_data_tx, log_data_rx) = mpsc::channel::<LogLeaf>(4);

    let handle = tokio::spawn(async move {
        let Input { mut log, mut log_rx } = input;
        while let Some(leaf) = log_rx.recv().await {
            log.push(leaf.encode());
    
            let checkpoint = log.checkpoint();
            let log_root: DynHash = checkpoint.root().into();
            let log_length = checkpoint.length() as u32;
    
            log_data_tx.send(leaf.clone());
            summary_tx.send(Summary { leaf, log_root, log_length }).await.unwrap();
        }

        log
    });

    Output {
        summary_rx,
        log_data_rx,
        handle
    }
}