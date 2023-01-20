use std::fmt;

use crate::{operator::OperatorRecord, package::PackageRecord, Encode, Signable, ProtoEnvelope};
use serde::{Deserialize, Serialize};
use warg_crypto::hash::{DynHash, Hash, SupportedDigest};

pub struct SimpleEncoder {
    bytes: Vec<u8>,
}

impl SimpleEncoder {
    pub fn new(prefix: &[u8]) -> SimpleEncoder {
        Self {
            bytes: Vec::from(prefix),
        }
    }

    pub fn append_unsigned(&mut self, i: u64) {
        leb128::write::unsigned(&mut self.bytes, i).unwrap();
    }

    fn append_str(&mut self, s: &str) {
        self.bytes.extend_from_slice(s.as_bytes());
    }

    pub fn append_len_str(&mut self, s: &str) {
        self.append_unsigned(s.len() as u64);
        self.append_str(s);
    }

    pub fn finalize(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapCheckpoint {
    pub log_root: DynHash,
    pub log_length: u32,
    pub map_root: DynHash,
}

impl Signable for MapCheckpoint {
    const PREFIX: &'static [u8] = b"WARG-MAP-CHECKPOINT-SIGNATURE-V0";
}

impl Encode for MapCheckpoint {
    fn encode(&self) -> Vec<u8> {
        let mut checkpoint = SimpleEncoder::new(b"WARG-MAP-CHECKPOINT-V0");

        checkpoint.append_unsigned(self.log_length as u64);

        let log_root = self.log_root.to_string();
        checkpoint.append_len_str(&log_root);

        let map_root = self.map_root.to_string();
        checkpoint.append_len_str(&map_root);

        checkpoint.finalize()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MapLeaf {
    pub record_id: RecordId,
}

impl Encode for MapLeaf {
    fn encode(&self) -> Vec<u8> {
        let mut leaf = SimpleEncoder::new(b"WARG-MAP-LEAF-V0");

        let record_id = self.record_id.0.to_string();
        leaf.append_len_str(&record_id);

        leaf.finalize()
    }
}

// #[derive(Debug, Clone, Hash, PartialEq, Eq)]
// pub struct LogCheckpoint {
//     pub log_root: DynHash,
//     pub log_length: u32,
// }

// impl Encode for LogCheckpoint {
//     fn encode(&self) -> Vec<u8> {
//         let mut checkpoint = SimpleEncoder::new(b"WARG-LOG-CHECKPOINT-V0");

//         checkpoint.append_unsigned(self.log_length as u64);

//         let log_root = self.log_root.to_string();
//         checkpoint.append_len_str(&log_root);

//         checkpoint.finalize()
//     }
// }

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LogLeaf {
    pub log_id: LogId,
    pub record_id: RecordId,
}

impl Encode for LogLeaf {
    fn encode(&self) -> Vec<u8> {
        let mut leaf = SimpleEncoder::new(b"WARG-MAP-LEAF-V0");

        let log_id = self.log_id.0.to_string();
        leaf.append_len_str(&log_id);

        let record_id = self.record_id.0.to_string();
        leaf.append_len_str(&record_id);

        leaf.finalize()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LogId(DynHash);

impl LogId {
    pub fn operator_log<D: SupportedDigest>() -> Self {
        let mut d = D::new();
        d.update(b"WARG-OPERATOR-LOG-ID-V0");
        let hash: Hash<D> = d.finalize().into();
        Self(hash.into())
    }

    pub fn package_log<D: SupportedDigest>(name: &str) -> Self {
        let mut d = D::new();
        d.update(b"WARG-PACKAGE-LOG-ID-V0:");
        d.update(name.as_bytes());
        let hash: Hash<D> = d.finalize().into();
        Self(hash.into())
    }
}

impl AsRef<[u8]> for LogId {
    fn as_ref(&self) -> &[u8] {
        self.0.bytes()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RecordId(DynHash);

impl RecordId {
    pub fn operator_record<D: SupportedDigest>(record: &ProtoEnvelope<OperatorRecord>) -> Self {
        let mut d = D::new();
        d.update(b"WARG-OPERATOR-LOG-RECORD-V0:");
        d.update(record.content_bytes());
        let hash: Hash<D> = d.finalize().into();
        Self(hash.into())
    }

    pub fn package_record<D: SupportedDigest>(record: &ProtoEnvelope<PackageRecord>) -> Self {
        let mut d = D::new();
        d.update(b"WARG-PACKAGE-LOG-RECORD-V0:");
        d.update(record.content_bytes());
        let hash: Hash<D> = d.finalize().into();
        Self(hash.into())
    }
}

impl fmt::Display for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<DynHash> for RecordId {
    fn from(value: DynHash) -> Self {
        Self(value)
    }
}
