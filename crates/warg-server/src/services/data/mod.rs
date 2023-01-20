use tokio::{sync::mpsc::Receiver, task::JoinHandle};
use warg_protocol::registry::LogLeaf;

use super::transparency::VerifiableMap;

mod log;
mod map;

pub use log::{LogData, ProofLog};
pub use map::MapData;

pub struct Input {
    pub log: log::ProofLog,
    pub log_rx: Receiver<LogLeaf>,
    pub maps: Vec<VerifiableMap>,
    pub map_rx: Receiver<VerifiableMap>,
}

pub struct Output {
    pub map_data: map::MapData,
    pub log_data: log::LogData,

    pub map_data_handle: JoinHandle<()>,
    pub log_data_handle: JoinHandle<()>,
}

pub fn process(input: Input) -> Output {
    let Input {
        log,
        log_rx,
        maps,
        map_rx,
    } = input;

    let log_input = log::Input { log, log_rx };
    let log_output = log::process(log_input);

    let map_input = map::Input { maps, map_rx };
    let map_output = map::process(map_input);

    Output {
        log_data: log_output.data,
        map_data: map_output.data,
        log_data_handle: log_output.handle,
        map_data_handle: map_output.handle,
    }
}
