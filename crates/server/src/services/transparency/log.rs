use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::registry::LogLeaf;
use warg_transparency::log::{LogBuilder, StackLog};

pub type VerifiableLog = StackLog<Sha256, LogLeaf>;

pub struct Input {
    pub log: VerifiableLog,
    pub log_rx: Receiver<LogLeaf>,
}

pub struct Output {
    pub summary_rx: Receiver<Summary>,
    pub log_data_rx: Receiver<LogLeaf>,
    pub handle: JoinHandle<VerifiableLog>,
}

#[derive(Debug)]
pub struct Summary {
    pub leaf: LogLeaf,
    pub log_root: DynHash,
    pub log_length: u32,
}

pub fn process(input: Input) -> Output {
    let (summary_tx, summary_rx) = mpsc::channel::<Summary>(4);
    let (log_data_tx, log_data_rx) = mpsc::channel::<LogLeaf>(4);

    let handle = tokio::spawn(async move {
        let Input {
            mut log,
            mut log_rx,
        } = input;
        while let Some(leaf) = log_rx.recv().await {
            log.push(&leaf);

            let checkpoint = log.checkpoint();
            let log_root: DynHash = checkpoint.root().into();
            let log_length = checkpoint.length() as u32;

            log_data_tx.send(leaf.clone()).await.unwrap();
            summary_tx
                .send(Summary {
                    leaf,
                    log_root,
                    log_length,
                })
                .await
                .unwrap();
        }

        log
    });

    Output {
        summary_rx,
        log_data_rx,
        handle,
    }
}
