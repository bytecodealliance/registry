use std::collections::HashMap;

use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;

use super::validator::ValidatorFactory;
use super::{validator::ValidatorService, PublishInfo};

pub struct PublishService {
    mailbox: mpsc::Sender<Message>,
    handle: JoinHandle<()>,
}

#[derive(Debug)]
enum Message {
    Publish { info: PublishInfo },
}

impl PublishService {
    pub fn new(v_factory: ValidatorFactory) -> Self {
        let (mailbox, rx) = mpsc::channel::<Message>(4);
        let handle = tokio::spawn(async move {
            Self::process(rx, v_factory).await;
        });

        Self { mailbox, handle }
    }

    async fn process(mut rx: Receiver<Message>, v_factory: ValidatorFactory) {
        let mut validators: HashMap<_, ValidatorService> = HashMap::new();

        while let Some(request) = rx.recv().await {
            match request {
                Message::Publish { info } => {
                    if let Some(validator) = validators.get(&info.package_id) {
                        validator.validate(info).await;
                    } else {
                        let validator = v_factory.create();
                        let package_id = info.package_id.clone();
                        validator.validate(info).await;
                        validators.insert(package_id, validator);
                    }
                }
            }
        }
    }
}

impl PublishService {
    pub async fn publish(&self, info: PublishInfo) {
        self.mailbox.send(Message::Publish { info }).await.unwrap();
    }
}
