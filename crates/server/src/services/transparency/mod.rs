use tokio::{sync::mpsc::Receiver, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use warg_crypto::signing;
use warg_protocol::registry::LogLeaf;

mod log;
mod map;
mod sign;

pub use log::VerifiableLog;
pub use map::VerifiableMap;
pub use sign::Signature;

pub struct Input {
    pub token: CancellationToken,
    pub log: log::VerifiableLog,
    pub map: map::VerifiableMap,
    pub leaves: Vec<LogLeaf>,
    pub signing_key: signing::PrivateKey,
    pub log_rx: Receiver<LogLeaf>,
}

pub struct Output {
    pub log_rx: Receiver<LogLeaf>,
    pub map_rx: Receiver<map::VerifiableMap>,
    pub signature_rx: Receiver<sign::Signature>,
    pub log_handle: JoinHandle<()>,
    pub map_handle: JoinHandle<()>,
    pub sign_handle: JoinHandle<()>,
}

pub fn spawn(input: Input) -> Output {
    let Input {
        token,
        log,
        map,
        leaves,
        log_rx,
        signing_key,
    } = input;

    let log = log::spawn(log::Input {
        token: token.clone(),
        log,
        log_rx,
    });
    let map = map::spawn(map::Input {
        token: token.clone(),
        map,
        leaves,
        log_summary_rx: log.log_summary_rx,
    });
    let sign = sign::spawn(sign::Input {
        token,
        signing_key,
        map_summary_rx: map.map_summary_rx,
    });

    Output {
        log_rx: log.log_rx,
        map_rx: map.map_rx,
        signature_rx: sign.signature_rx,
        log_handle: log.handle,
        map_handle: map.handle,
        sign_handle: sign.handle,
    }
}
