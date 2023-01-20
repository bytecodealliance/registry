use std::sync::Arc;

use tokio::{sync::{mpsc::Receiver, RwLock}, task::JoinHandle};
use warg_protocol::registry::LogLeaf;

use super::transparency::VerifiableMap;

mod log;
mod map;

pub use log::{LogData, ProofLog};
pub use map::MapData;

pub struct Input {
    pub log_data: LogData,
    pub log_rx: Receiver<LogLeaf>,
    pub map_data: MapData,
    pub map_rx: Receiver<VerifiableMap>,
}

pub struct Output {
    pub map_data: Arc<RwLock<map::MapData>>,
    pub log_data: Arc<RwLock<log::LogData>>,

    pub map_data_handle: JoinHandle<()>,
    pub log_data_handle: JoinHandle<()>,
}

pub fn process(input: Input) -> Output {
    let Input {
        log_data,
        log_rx,
        map_data,
        map_rx,
    } = input;

    let log_input = log::Input { data: log_data, log_rx };
    let log_output = log::process(log_input);

    let map_input = map::Input { data: map_data, map_rx };
    let map_output = map::process(map_input);

    Output {
        log_data: log_output.data,
        map_data: map_output.data,
        log_data_handle: log_output.handle,
        map_data_handle: map_output.handle,
    }
}
