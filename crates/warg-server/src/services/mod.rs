pub mod core;
pub mod data;
pub mod transparency;

use std::{sync::Arc, time::SystemTime};

use self::{
    core::{CoreService, State},
    data::{log, map},
    transparency::{VerifiableLog, VerifiableMap},
};
use forrest::log::LogBuilder;
use tokio::sync::mpsc;
use warg_crypto::{
    hash::{HashAlgorithm, Sha256},
    signing::PrivateKey,
};
use warg_protocol::{
    operator,
    registry::{LogId, LogLeaf, MapCheckpoint, MapLeaf, RecordId},
    ProtoEnvelope, SerdeEnvelope,
};

pub fn init(signing_key: PrivateKey) -> (Arc<CoreService>, data::Output) {
    let (transparency_tx, transparency_rx) = mpsc::channel(4);

    let init_envelope = init_envelope(&signing_key);
    let log_id = LogId::operator_log::<Sha256>();
    let record_id = RecordId::operator_record::<Sha256>(&init_envelope);

    let log_leaf = LogLeaf {
        log_id: log_id.clone(),
        record_id: record_id.clone(),
    };
    let map_leaf = MapLeaf { record_id };

    let mut log = VerifiableLog::default();
    log.push(&log_leaf);

    let map = VerifiableMap::default();
    let map = map.insert(log_id, map_leaf.clone());

    let log_checkpoint = log.checkpoint();
    let checkpoint = MapCheckpoint {
        log_root: log_checkpoint.root().into(),
        log_length: log_checkpoint.length() as u32,
        map_root: map.root().clone().into(),
    };
    let checkpoint = SerdeEnvelope::signed_contents(&signing_key, checkpoint).unwrap();

    let input = transparency::Input {
        log,
        map,
        private_key: signing_key,
        log_rx: transparency_rx,
    };

    let transparency = transparency::process(input);

    let log_data = log::ProofData::new(log_leaf.clone());
    let map_data = map::MapData::new(map_leaf.clone());

    let input = data::Input {
        log_data,
        log_rx: transparency.log_data,
        map_data,
        map_rx: transparency.map_data,
    };

    let data = data::process(input);

    let initial_state = State::new(checkpoint, init_envelope);
    let core = Arc::new(CoreService::start(initial_state, transparency_tx));

    let mut signatures = transparency.signatures;
    let sig_core = core.clone();
    tokio::spawn(async move {
        while let Some(sig) = signatures.recv().await {
            sig_core
                .as_ref()
                .new_checkpoint(sig.envelope, sig.leaves)
                .await;
        }
    });

    (core, data)
}

fn init_envelope(signing_key: &PrivateKey) -> ProtoEnvelope<operator::OperatorRecord> {
    let init_record = operator::OperatorRecord {
        prev: None,
        version: 0,
        timestamp: SystemTime::now(),
        entries: vec![operator::OperatorEntry::Init {
            hash_algorithm: HashAlgorithm::Sha256,
            key: signing_key.public_key(),
        }],
    };
    ProtoEnvelope::signed_contents(&signing_key, init_record).unwrap()
}
