use tokio::{sync::mpsc::Receiver, task::JoinHandle};

use warg_protocol::{registry::LogLeaf, signing};

mod log;
mod map;
mod sign;

pub use log::VerifiableLog;
pub use map::VerifiableMap;
pub use sign::Signature;

pub struct Input {
    pub log: log::VerifiableLog,
    pub map: map::VerifiableMap,
    pub private_key: signing::PrivateKey,

    pub log_rx: Receiver<LogLeaf>,
}

pub struct Output {
    pub log_handle: JoinHandle<log::VerifiableLog>,
    pub map_handle: JoinHandle<map::VerifiableMap>,
    pub sign_handle: JoinHandle<()>,

    pub log_data: Receiver<LogLeaf>,
    pub map_data: Receiver<map::VerifiableMap>,
    pub signatures: Receiver<sign::Signature>,
}

pub async fn process(input: Input) -> Output {
    let Input {
        log,
        map,
        log_rx,
        private_key,
    } = input;

    let log_output = log::process(log::Input { log, log_rx }).await;
    let map_output = map::process(map::Input {
        map,
        map_rx: log_output.summary_rx,
    })
    .await;
    let sign_output = sign::process(sign::Input {
        private_key,
        sign_rx: map_output.summary_rx,
    })
    .await;

    Output {
        log_handle: log_output.handle,
        map_handle: map_output.handle,
        sign_handle: sign_output.handle,

        log_data: log_output.log_data_rx,
        map_data: map_output.map_data_rx,
        signatures: sign_output.signatures,
    }
}
