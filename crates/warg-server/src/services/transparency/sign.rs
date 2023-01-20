use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;

use warg_protocol::registry::{LogLeaf, MapCheckpoint};
use warg_protocol::{signing, Envelope};

use super::map;

pub struct Input {
    pub private_key: signing::PrivateKey,
    pub sign_rx: Receiver<map::Summary>,
}

pub struct Output {
    pub signatures: Receiver<Signature>,
    pub handle: JoinHandle<()>,
}

#[derive(Debug)]
pub struct Signature {
    pub leaves: Vec<LogLeaf>,
    pub envelope: Envelope<MapCheckpoint>,
}

pub fn process(input: Input) -> Output {
    let (summary_tx, signatures) = mpsc::channel::<Signature>(4);

    let handle = tokio::spawn(async move {
        let Input {
            private_key,
            mut sign_rx,
        } = input;

        while let Some(message) = sign_rx.recv().await {
            let map::Summary { leaves, checkpoint } = message;
            let envelope = Envelope::signed_contents(&private_key, checkpoint).unwrap();
            summary_tx
                .send(Signature { leaves, envelope })
                .await
                .unwrap();
        }
    });

    Output { signatures, handle }
}
