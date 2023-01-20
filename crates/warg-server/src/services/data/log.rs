use std::sync::Arc;

use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use forrest::log::{LogBuilder, VecLog};
use warg_crypto::hash::Sha256;
use warg_protocol::registry::LogLeaf;
use warg_protocol::Encode;

pub type ProofLog = VecLog<Sha256>;

pub type LogData = Arc<RwLock<ProofLog>>;

pub struct Input {
    pub log: ProofLog,
    pub log_rx: Receiver<LogLeaf>,
}

pub struct Output {
    pub data: LogData,
    pub handle: JoinHandle<()>,
}

pub fn process(input: Input) -> Output {
    let Input { log, mut log_rx } = input;
    let data = Arc::new(RwLock::new(log));
    let processor_data = data.clone();

    let handle = tokio::spawn(async move {
        let data = processor_data;

        while let Some(leaf) = log_rx.recv().await {
            let mut log = data.as_ref().blocking_write();
            log.push(leaf.encode());
            drop(log);
        }
    });

    Output { data, handle }
}
