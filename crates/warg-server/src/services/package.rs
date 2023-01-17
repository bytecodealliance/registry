use std::collections::HashMap;

use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use warg_protocol::registry::{LogId, RecordId};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum PackageRecordStatus {
    #[default]
    Processing,
    Validated,
    Published {
        record_url: String,
    },
    Rejected {
        reason: String,
    },
}

pub struct PackageService {
    mailbox: mpsc::Sender<Message>,
    handle: JoinHandle<()>,
}

#[derive(Debug)]
enum Message {
    SetRecordStatus {
        package_id: LogId,
        record_id: RecordId,
        status: PackageRecordStatus,
    },
    GetRecordStatus {
        package_id: LogId,
        record_id: RecordId,
        response: oneshot::Sender<Option<PackageRecordStatus>>,
    },
}

impl PackageService {
    pub fn new() -> Self {
        let (mailbox, rx) = mpsc::channel::<Message>(4);
        let handle = tokio::spawn(async move {
            Self::process(rx).await;
        });

        Self { mailbox, handle }
    }

    async fn process(mut rx: Receiver<Message>) {
        let mut record_statuses: HashMap<(LogId, RecordId), PackageRecordStatus> = HashMap::new();

        while let Some(request) = rx.recv().await {
            match request {
                Message::SetRecordStatus {
                    package_id,
                    record_id,
                    status,
                } => {
                    record_statuses.insert((package_id, record_id), status);
                }
                Message::GetRecordStatus {
                    package_id,
                    record_id,
                    response,
                } => {
                    response
                        .send(record_statuses.get(&(package_id, record_id)).cloned())
                        .unwrap();
                }
            }
        }
    }
}

impl PackageService {
    pub async fn set_record_status(
        &self,
        package_id: LogId,
        record_id: RecordId,
        status: PackageRecordStatus,
    ) {
        self.mailbox
            .send(Message::SetRecordStatus {
                package_id,
                record_id,
                status,
            })
            .await
            .unwrap();
    }

    pub async fn get_record_status(
        &self,
        package_id: LogId,
        record_id: RecordId,
    ) -> Option<PackageRecordStatus> {
        let (tx, rx) = oneshot::channel();
        self.mailbox
            .send(Message::GetRecordStatus {
                package_id,
                record_id,
                response: tx,
            })
            .await
            .unwrap();
        rx.await.unwrap()
    }
}
