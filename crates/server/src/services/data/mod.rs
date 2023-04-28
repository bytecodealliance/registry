use super::transparency::VerifiableMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::{
    sync::{mpsc::Receiver, RwLock},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use warg_crypto::hash::{Hash, Sha256};
use warg_protocol::registry::{LogId, LogLeaf};

pub mod log;
pub mod map;

#[derive(Debug, Error)]
pub enum DataServiceError {
    #[error("root `{0}` was not found")]
    RootNotFound(Hash<Sha256>),
    #[error("log leaf `{}:{}` was not found", .0.log_id, .0.record_id)]
    LeafNotFound(LogLeaf),
    #[error("failed to bundle proofs: {0}")]
    BundleFailure(anyhow::Error),
    #[error("failed to prove inclusion of package `{0}`")]
    PackageNotIncluded(LogId),
    #[error("failed to prove inclusion: found root `{found}` but was given root `{root}`")]
    IncorrectProof {
        root: Hash<Sha256>,
        found: Hash<Sha256>,
    },
}

pub struct Input {
    pub token: CancellationToken,
    pub log_data: log::LogData,
    pub log_rx: Receiver<LogLeaf>,
    pub map_data: map::MapData,
    pub map_rx: Receiver<VerifiableMap>,
}

pub struct Output {
    pub log_data: Arc<RwLock<log::LogData>>,
    pub map_data: Arc<RwLock<map::MapData>>,
    pub log_handle: JoinHandle<()>,
    pub map_handle: JoinHandle<()>,
}

pub fn spawn(input: Input) -> Output {
    let Input {
        token,
        log_data,
        log_rx,
        map_data,
        map_rx,
    } = input;

    let log = log::spawn(log::Input {
        token: token.clone(),
        data: log_data,
        log_rx,
    });

    let map = map::spawn(map::Input {
        token,
        data: map_data,
        map_rx,
    });

    Output {
        log_data: log.data,
        map_data: map.data,
        log_handle: log.handle,
        map_handle: map.handle,
    }
}
