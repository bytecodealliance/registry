use std::sync::Arc;

use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::services::transparency::VerifiableMap;

type MapData = Arc<RwLock<Vec<VerifiableMap>>>;

pub struct Input {
    pub maps: Vec<VerifiableMap>,
    pub map_rx: Receiver<VerifiableMap>,
}

pub struct Output {
    pub data: MapData,
    _handle: JoinHandle<()>,
}

pub async fn process(input: Input) -> Output {
    let Input { maps, mut map_rx } = input;
    let data = Arc::new(RwLock::new(maps));
    let processor_data = data.clone();

    let _handle = tokio::spawn(async move {
        let data = processor_data;

        while let Some(map) = map_rx.recv().await {
            let mut maps = data.as_ref().blocking_write();
            maps.push(map);
            drop(maps);
        }
    });

    Output { data, _handle }
}
