use tokio::{
    sync::mpsc::{self, Receiver},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use warg_crypto::signing;
use warg_protocol::{
    registry::{LogLeaf, MapCheckpoint},
    SerdeEnvelope,
};

use super::map;

pub struct Input {
    pub token: CancellationToken,
    pub signing_key: signing::PrivateKey,
    pub map_summary_rx: Receiver<map::Summary>,
}

pub struct Output {
    pub signature_rx: Receiver<Signature>,
    pub handle: JoinHandle<()>,
}

#[derive(Debug)]
pub struct Signature {
    pub leaves: Vec<LogLeaf>,
    pub envelope: SerdeEnvelope<MapCheckpoint>,
}

pub fn spawn(input: Input) -> Output {
    let (signature_tx, signature_rx) = mpsc::channel::<Signature>(4);

    let handle = tokio::spawn(async move {
        let Input {
            token,
            signing_key,
            mut map_summary_rx,
        } = input;

        loop {
            tokio::select! {
                summary = map_summary_rx.recv() => {
                    if let Some(map::Summary { leaves, checkpoint }) = summary {
                        let envelope = SerdeEnvelope::signed_contents(&signing_key, checkpoint).unwrap();
                        signature_tx
                            .send(Signature { leaves, envelope })
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
        signature_rx,
        handle,
    }
}
