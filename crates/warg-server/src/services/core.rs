use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::registry::LogLeaf;
use warg_protocol::{
    operator, package,
    registry::{LogId, MapCheckpoint, RecordId},
    Envelope,
};

#[derive(Clone, Debug, Default)]
pub struct State {
    checkpoints: Vec<Arc<Envelope<MapCheckpoint>>>,
    operator_state: Arc<Mutex<OperatorInfo>>,
    package_states: HashMap<LogId, Arc<Mutex<PackageInfo>>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct OperatorInfo {
    validator: operator::Validator,
    log: Vec<Arc<Envelope<operator::OperatorRecord>>>,
    records: HashMap<RecordId, OperatorRecordInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OperatorRecordInfo {
    record: Arc<Envelope<operator::OperatorRecord>>,
    state: RecordState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PackageInfo {
    id: LogId,
    name: String,
    validator: package::Validator,
    log: Vec<Arc<Envelope<package::PackageRecord>>>,
    pending_record: Option<RecordId>,
    records: HashMap<RecordId, PackageRecordInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageRecordInfo {
    pub record: Arc<Envelope<package::PackageRecord>>,
    pub content_sources: Arc<Vec<ContentSource>>,
    pub state: RecordState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentSource {
    pub content_digest: DynHash,
    pub kind: ContentSourceKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentSourceKind {
    HttpAnonymous { url: String },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordState {
    #[default]
    Unknown,
    Processing,
    Published {
        checkpoint: Arc<Envelope<MapCheckpoint>>,
    },
    Rejected {
        reason: String,
    },
}

pub struct CoreService {
    mailbox: mpsc::Sender<Message>,
    handle: JoinHandle<State>,
}

#[derive(Debug)]
enum Message {
    SubmitPackageRecord {
        package_name: String,
        record: Arc<Envelope<package::PackageRecord>>,
        content_sources: Vec<ContentSource>,
        response: oneshot::Sender<RecordState>,
    },
    GetPackageRecordStatus {
        package_id: LogId,
        record_id: RecordId,
        response: oneshot::Sender<RecordState>,
    },
    GetPackageRecordInfo {
        package_id: LogId,
        record_id: RecordId,
        response: oneshot::Sender<Option<PackageRecordInfo>>,
    },
    NewCheckpoint {
        checkpoint: Arc<Envelope<MapCheckpoint>>,
        leaves: Vec<LogLeaf>,
    },
}

impl CoreService {
    pub fn new(initial_state: State, transparency_tx: Sender<LogLeaf>) -> Self {
        let (mailbox, rx) = mpsc::channel::<Message>(4);
        let handle =
            tokio::spawn(async move { Self::process(initial_state, rx, transparency_tx).await });

        Self { mailbox, handle }
    }

    async fn process(
        initial_state: State,
        mut rx: Receiver<Message>,
        transparency_tx: Sender<LogLeaf>,
    ) -> State {
        let mut state = initial_state;

        while let Some(request) = rx.recv().await {
            match request {
                Message::SubmitPackageRecord {
                    package_name,
                    record,
                    content_sources,
                    response,
                } => {
                    let package_id = LogId::package_log::<Sha256>(&package_name);
                    let package_info = state
                        .package_states
                        .entry(package_id.clone())
                        .or_insert_with(|| {
                            Arc::new(Mutex::new(PackageInfo {
                                id: package_id,
                                name: package_name,
                                validator: Default::default(),
                                log: Default::default(),
                                pending_record: Default::default(),
                                records: Default::default(),
                            }))
                        })
                        .clone();
                    let transparency_tx = transparency_tx.clone();
                    tokio::spawn(async move {
                        new_record(
                            package_info,
                            record,
                            content_sources,
                            response,
                            transparency_tx,
                        )
                        .await
                    });
                },
                Message::GetPackageRecordStatus {
                    package_id,
                    record_id,
                    response,
                } => {
                    if let Some(package_info) = state.package_states.get(&package_id).cloned() {
                        tokio::spawn(async move {
                            let info = package_info.as_ref().blocking_lock();
                            if let Some(record_info) = info.records.get(&record_id) {
                                response.send(record_info.state.clone()).unwrap();
                            } else {
                                response.send(RecordState::Unknown).unwrap();
                            }
                        });
                    } else {
                        response.send(RecordState::Unknown).unwrap();
                    }
                },
                Message::GetPackageRecordInfo { package_id, record_id, response } => {
                    if let Some(package_info) = state.package_states.get(&package_id).cloned() {
                        tokio::spawn(async move {
                            let info = package_info.as_ref().blocking_lock();
                            if let Some(record_info) = info.records.get(&record_id) {
                                response.send(Some(record_info.clone())).unwrap();
                            } else {
                                response.send(None).unwrap();
                            }
                        });
                    } else {
                        response.send(None).unwrap();
                    }
                },
                Message::NewCheckpoint { checkpoint, leaves } => {
                    for leaf in leaves {
                        let package_info = state.package_states.get(&leaf.log_id).unwrap().clone();
                        let checkpoint_clone = checkpoint.clone();
                        tokio::spawn(async move {
                            mark_published(package_info, leaf.record_id, checkpoint_clone).await
                        });
                    }
                }
            }
        }

        state
    }
}

async fn new_record(
    package_info: Arc<Mutex<PackageInfo>>,
    record: Arc<Envelope<package::PackageRecord>>,
    content_sources: Vec<ContentSource>,
    response: oneshot::Sender<RecordState>,
    transparency_tx: Sender<LogLeaf>,
) {
    let mut info = package_info.as_ref().blocking_lock();

    let record_id = RecordId::package_record::<Sha256>(&record);
    let mut hypothetical = info.validator.clone();
    match hypothetical.validate(&record) {
        Ok(contents) => {
            let provided_contents: HashSet<DynHash> = content_sources
                .iter()
                .map(|source| source.content_digest.clone())
                .collect();
            for needed_content in contents {
                if !provided_contents.contains(&needed_content) {
                    let state = RecordState::Rejected {
                        reason: format!("Needed content {} but not provided", needed_content),
                    };
                    response.send(state).unwrap();
                    return;
                }
            }

            let state = RecordState::Processing;
            let record_info = PackageRecordInfo {
                record: record.clone(),
                content_sources: Arc::new(content_sources),
                state: state.clone(),
            };

            transparency_tx
                .send(LogLeaf {
                    log_id: info.id.clone(),
                    record_id: record_id.clone(),
                })
                .await
                .unwrap();

            info.validator = hypothetical;
            info.log.push(record);
            info.records.insert(record_id, record_info);

            response.send(state).unwrap();
        }
        Err(error) => {
            let reason = error.to_string();
            let state = RecordState::Rejected { reason };
            let record_info = PackageRecordInfo {
                record,
                content_sources: Arc::new(content_sources),
                state: state.clone(),
            };
            info.records.insert(record_id, record_info);

            response.send(state).unwrap();
        }
    };
}

async fn mark_published(
    package_info: Arc<Mutex<PackageInfo>>,
    record_id: RecordId,
    checkpoint: Arc<Envelope<MapCheckpoint>>,
) {
    let mut info = package_info.as_ref().blocking_lock();

    info.records.get_mut(&record_id).unwrap().state = RecordState::Published { checkpoint };
}

impl CoreService {
    pub async fn submit_package_record(
        &self,
        package_name: String,
        record: Arc<Envelope<package::PackageRecord>>,
        content_sources: Vec<ContentSource>,
    ) -> RecordState {
        let (tx, rx) = oneshot::channel();
        self.mailbox
            .send(Message::SubmitPackageRecord {
                package_name,
                record,
                content_sources,
                response: tx,
            })
            .await
            .unwrap();

        rx.await.unwrap()
    }

    pub async fn get_package_record_status(&self, package_name: String, record_id: RecordId) -> RecordState {
        let package_id = LogId::package_log::<Sha256>(&package_name);
        let (tx, rx) = oneshot::channel();
        self.mailbox
            .send(Message::GetPackageRecordStatus {
                package_id,
                record_id,
                response: tx,
            })
            .await
            .unwrap();

        rx.await.unwrap()
    }

    pub async fn get_package_record_info(&self, package_name: String, record_id: RecordId) -> Option<PackageRecordInfo> {
        let package_id = LogId::package_log::<Sha256>(&package_name);
        let (tx, rx) = oneshot::channel();
        self.mailbox
            .send(Message::GetPackageRecordInfo {
                package_id,
                record_id,
                response: tx,
            })
            .await
            .unwrap();

        rx.await.unwrap()
    }

    pub async fn new_checkpoint(&self, checkpoint: Envelope<MapCheckpoint>, leaves: Vec<LogLeaf>) {
        self.mailbox
            .send(Message::NewCheckpoint {
                checkpoint: Arc::new(checkpoint),
                leaves,
            })
            .await
            .unwrap();
    }
}
