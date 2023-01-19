use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;

use warg_protocol::{signing, Envelope};
use warg_protocol::registry::{MapCheckpoint, LogLeaf};

use super::map;

pub struct Input {
    pub private_key: signing::PrivateKey,
    pub sign_rx: Receiver<map::Summary>
}

pub struct Output {
    pub summary_rx: Receiver<Summary>,
    _handle: JoinHandle<()>
}

#[derive(Debug)]
pub struct Summary {
    pub leaves: Vec<LogLeaf>,
    pub envelope: Envelope<MapCheckpoint>
}

pub async fn process(input: Input) -> Output {
    let (summary_tx, summary_rx) = mpsc::channel::<Summary>(4);

    let _handle = tokio::spawn(async move {
        let Input { private_key, mut sign_rx } = input;

        while let Some(message) = sign_rx.recv().await {
            let map::Summary { leaves, checkpoint } = message;
            let envelope = Envelope::signed_contents(&private_key, checkpoint).unwrap();
            summary_tx.send(Summary { leaves, envelope }).await.unwrap();
        }
    });

    Output {
        summary_rx,
        _handle
    }
}