use std::sync::Arc;

use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::services::transparency::VerifiableMap;

pub struct Input {
    pub data: MapData,
    pub map_rx: Receiver<VerifiableMap>,
}

pub struct Output {
    pub data: Arc<RwLock<MapData>>,
    pub handle: JoinHandle<()>,
}

#[derive(Default)]
pub struct MapData {
    maps: Vec<VerifiableMap>,
}

pub fn process(input: Input) -> Output {
    let Input { data, mut map_rx } = input;
    let data = Arc::new(RwLock::new(data));
    let processor_data = data.clone();

    let handle = tokio::spawn(async move {
        let data = processor_data;

        while let Some(map) = map_rx.recv().await {
            let mut data = data.as_ref().blocking_write();
            data.maps.push(map);
            drop(data);
        }
    });

    Output { data, handle }
}
