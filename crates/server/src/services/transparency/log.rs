use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::registry::LogLeaf;
use warg_transparency::log::{LogBuilder, StackLog};

pub type VerifiableLog = StackLog<Sha256, LogLeaf>;

pub struct Input {
    pub token: CancellationToken,
    pub log: VerifiableLog,
    pub log_rx: Receiver<LogLeaf>,
}

pub struct Output {
    pub log_summary_rx: Receiver<Summary>,
    pub log_rx: Receiver<LogLeaf>,
    pub handle: JoinHandle<()>,
}

#[derive(Debug)]
pub struct Summary {
    pub leaf: LogLeaf,
    pub log_root: DynHash,
    pub log_length: u32,
}

pub fn spawn(input: Input) -> Output {
    let (log_summary_tx, log_summary_rx) = mpsc::channel::<Summary>(4);
    let (log_tx, log_rx) = mpsc::channel::<LogLeaf>(4);

    let handle = tokio::spawn(async move {
        let Input {
            token,
            mut log,
            mut log_rx,
        } = input;

        loop {
            tokio::select! {
                leaf = log_rx.recv() => {
                    if let Some(leaf) = leaf {
                        log.push(&leaf);

                        let checkpoint = log.checkpoint();
                        let log_root: DynHash = checkpoint.root().into();
                        let log_length = checkpoint.length() as u32;

                        log_tx.send(leaf.clone()).await.unwrap();
                        log_summary_tx
                            .send(Summary {
                                leaf,
                                log_root,
                                log_length,
                            })
                            .await
                            .unwrap();
                    } else {
                        break;
                    }
                },
                _ = token.cancelled() => {
                    break;
                }
            }
        }
    });

    Output {
        log_summary_rx,
        log_rx,
        handle,
    }
}
