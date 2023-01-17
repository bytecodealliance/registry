use warg_crypto::hash::{DynHash, SupportedDigest, Digest, Output, Hash};
use crate::{Encode, Signable, operator::model::OperatorRecord, Envelope, package::model::PackageRecord};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
        let mut checkpoint = Vec::new();
        checkpoint.extend_from_slice(b"WARG-MAP-CHECKPOINT-V0");

        // TODO: leb128 of log_length

        let log_root = self.log_root.to_string();
        let log_root_len = log_root.len();
        // TODO: leb128 of len(log_root_len)
        checkpoint.extend_from_slice(log_root.as_bytes());


        let map_root = self.map_root.to_string();
        let map_root_len = map_root.len();
        // TODO: leb128 of len(map_root_len)
        checkpoint.extend_from_slice(map_root.as_bytes());

        checkpoint
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MapLeaf {
    pub record_id: RecordId
}

impl Encode for MapLeaf {
    fn encode(&self) -> Vec<u8> {
        let mut leaf = Vec::new();
        leaf.extend_from_slice(b"WARG-MAP-LEAF-V0");

        let record_id = self.record_id.0.to_string();
        let record_id_len = record_id.len();
        // TODO: leb128 of len(record_id_len)
        leaf.extend_from_slice(record_id.as_bytes());

        leaf
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LogLeaf {
    pub log_id: LogId,
    pub record_id: RecordId
}

impl Encode for LogLeaf {
    fn encode(&self) -> Vec<u8> {
        let mut leaf = Vec::new();
        leaf.extend_from_slice(b"WARG-LOG-LEAF-V0");

        let log_id = self.log_id.0.to_string();
        let log_id_len = log_id.len();
        // TODO: leb128 of len(log_id_len)
        leaf.extend_from_slice(log_id.as_bytes());

        let record_id = self.record_id.0.to_string();
        let record_id_len = record_id.len();
        // TODO: leb128 of len(record_id_len)
        leaf.extend_from_slice(record_id.as_bytes());

        leaf
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

    pub fn package_log<D: SupportedDigest>(name: String) -> Self {
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
    pub fn operator_record<D: SupportedDigest>(record: &Envelope<OperatorRecord>) -> Self {
        let mut d = D::new();
        d.update(b"WARG-OPERATOR-LOG-RECORD-V0:");
        d.update(record.content_bytes());
        let hash: Hash<D> = d.finalize().into();
        Self(hash.into())
    }

    pub fn package_record<D: SupportedDigest>(record: &Envelope<PackageRecord>) -> Self {
        let mut d = D::new();
        d.update(b"WARG-PACKAGE-LOG-RECORD-V0:");
        d.update(record.content_bytes());
        let hash: Hash<D> = d.finalize().into();
        Self(hash.into())
    }
}
