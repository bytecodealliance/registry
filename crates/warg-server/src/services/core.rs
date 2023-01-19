use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use warg_crypto::hash::Sha256;
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
struct PackageRecordInfo {
    record: Arc<Envelope<package::PackageRecord>>,
    state: RecordState,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
        response: oneshot::Sender<RecordState>,
    },
    NewCheckpoint {
        checkpoint: Arc<Envelope<MapCheckpoint>>,
        leaves: Vec<LogLeaf>,
    },
    GetRecordStatus {
        package_id: LogId,
        record_id: RecordId,
        response: oneshot::Sender<RecordState>,
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
                        new_record(package_info, record, response, transparency_tx).await
                    });
                }
                Message::NewCheckpoint { checkpoint, leaves } => {
                    for leaf in leaves {
                        let package_info = state.package_states.get(&leaf.log_id).unwrap().clone();
                        let checkpoint_clone = checkpoint.clone();
                        tokio::spawn(async move {
                            mark_published(package_info, leaf.record_id, checkpoint_clone).await
                        });
                    }
                }
                Message::GetRecordStatus {
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
                }
            }
        }

        state
    }
}

async fn new_record(
    package_info: Arc<Mutex<PackageInfo>>,
    record: Arc<Envelope<package::PackageRecord>>,
    response: oneshot::Sender<RecordState>,
    transparency_tx: Sender<LogLeaf>,
) {
    let mut info = package_info.as_ref().blocking_lock();

    let record_id = RecordId::package_record::<Sha256>(&record);
    let mut hypothetical = info.validator.clone();
    let state = match hypothetical.validate(&record) {
        Ok(()) => {
            let state = RecordState::Processing;
            let record_info = PackageRecordInfo {
                record: record.clone(),
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

            state
        }
        Err(error) => {
            let reason = error.to_string();
            let state = RecordState::Rejected { reason };
            let record_info = PackageRecordInfo {
                record,
                state: state.clone(),
            };
            info.records.insert(record_id, record_info);

            state
        }
    };

    response.send(state).unwrap();
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
    pub async fn new_package_record(
        &self,
        package_name: String,
        record: Arc<Envelope<package::PackageRecord>>,
    ) -> RecordState {
        let (tx, rx) = oneshot::channel();
        self.mailbox
            .send(Message::SubmitPackageRecord {
                package_name,
                record,
                response: tx,
            })
            .await
            .unwrap();

        rx.await.unwrap()
    }
}
