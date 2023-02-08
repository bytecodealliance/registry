use std::fmt;

use crate::{operator::OperatorRecord, package::PackageRecord, ProtoEnvelope};
use serde::{Deserialize, Serialize};
use warg_crypto::hash::{DynHash, Hash, HashAlgorithm, SupportedDigest};
use warg_crypto::{prefix, ByteVisitor, Signable, VisitBytes};

use warg_crypto::prefix::VisitPrefixEncode;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapCheckpoint {
    pub log_root: DynHash,
    pub log_length: u32,
    pub map_root: DynHash,
}

impl Signable for MapCheckpoint {
    const PREFIX: &'static [u8] = b"WARG-MAP-CHECKPOINT-SIGNATURE-V0";
}

impl prefix::VisitPrefixEncode for MapCheckpoint {
    fn visit_pe<'a, BV: ?Sized + ByteVisitor>(
        &self,
        visitor: &mut prefix::PrefixEncodeVisitor<'a, BV>,
    ) {
        visitor.visit_str_raw("WARG-MAP-CHECKPOINT-V0");
        visitor.visit_unsigned(self.log_length as u64);
        visitor.visit_str(&self.log_root.to_string());
        visitor.visit_str(&self.map_root.to_string());
    }
}

// Manual impls of VisitBytes for VisitPrefixEncode to avoid conflict with blanket impls
impl VisitBytes for MapCheckpoint {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.visit_bv(visitor);
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MapLeaf {
    pub record_id: RecordId,
}

impl prefix::VisitPrefixEncode for MapLeaf {
    fn visit_pe<'a, BV: ?Sized + ByteVisitor>(
        &self,
        visitor: &mut prefix::PrefixEncodeVisitor<'a, BV>,
    ) {
        visitor.visit_str_raw("WARG-MAP-LEAF-V0");
        visitor.visit_str(&self.record_id.0.to_string());
    }
}

// Manual impls of VisitBytes for VisitPrefixEncode to avoid conflict with blanket impls
impl VisitBytes for MapLeaf {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.visit_bv(visitor);
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLeaf {
    pub log_id: LogId,
    pub record_id: RecordId,
}

impl prefix::VisitPrefixEncode for LogLeaf {
    fn visit_pe<'a, BV: ?Sized + ByteVisitor>(
        &self,
        visitor: &mut prefix::PrefixEncodeVisitor<'a, BV>,
    ) {
        visitor.visit_str_raw("WARG-LOG-LEAF-V0");
        visitor.visit_str(&self.log_id.0.to_string());
        visitor.visit_str(&self.record_id.0.to_string());
    }
}

// Manual impls of VisitBytes for VisitPrefixEncode to avoid conflict with blanket impls
impl VisitBytes for LogLeaf {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.visit_bv(visitor);
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogId(DynHash);

impl LogId {
    pub fn operator_log<D: SupportedDigest>() -> Self {
        let prefix: &[u8] = b"WARG-OPERATOR-LOG-ID-V0".as_slice();
        let hash: Hash<D> = Hash::of(&prefix);
        Self(hash.into())
    }

    pub fn package_log<D: SupportedDigest>(name: &str) -> Self {
        let prefix: &[u8] = b"WARG-PACKAGE-LOG-ID-V0:".as_slice();
        let hash: Hash<D> = Hash::of(&(prefix, name));
        Self(hash.into())
    }
}

impl VisitBytes for LogId {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        visitor.visit_bytes(self.0.bytes())
    }
}

impl From<LogId> for DynHash {
    fn from(id: LogId) -> Self {
        id.0
    }
}

impl AsRef<[u8]> for LogId {
    fn as_ref(&self) -> &[u8] {
        self.0.bytes()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordId(DynHash);

impl RecordId {
    pub fn algorithm(&self) -> HashAlgorithm {
        self.0.algorithm()
    }

    pub fn operator_record<D: SupportedDigest>(record: &ProtoEnvelope<OperatorRecord>) -> Self {
        let prefix: &[u8] = b"WARG-OPERATOR-LOG-RECORD-V0:".as_slice();
        let hash: Hash<D> = Hash::of(&(prefix, record.content_bytes()));
        Self(hash.into())
    }

    pub fn package_record<D: SupportedDigest>(record: &ProtoEnvelope<PackageRecord>) -> Self {
        let prefix: &[u8] = b"WARG-PACKAGE-LOG-RECORD-V0:".as_slice();
        let hash: Hash<D> = Hash::of(&(prefix, record.content_bytes()));
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
