use crate::{operator::OperatorRecord, package::PackageRecord, ProtoEnvelope};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;
use warg_crypto::hash::{AnyHash, Hash, HashAlgorithm, SupportedDigest};
use warg_crypto::prefix::VisitPrefixEncode;
use warg_crypto::{prefix, ByteVisitor, Signable, VisitBytes};
use wasmparser::names::KebabStr;

/// Type alias for registry log index
pub type RegistryIndex = usize;

/// Type alias for registry log length
pub type RegistryLen = RegistryIndex;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub log_root: AnyHash,
    pub log_length: RegistryLen,
    pub map_root: AnyHash,
}

impl prefix::VisitPrefixEncode for Checkpoint {
    fn visit_pe<BV: ?Sized + ByteVisitor>(&self, visitor: &mut prefix::PrefixEncodeVisitor<BV>) {
        visitor.visit_str_raw("WARG-CHECKPOINT-V0");
        visitor.visit_unsigned(self.log_length as u64);
        visitor.visit_str(&self.log_root.to_string());
        visitor.visit_str(&self.map_root.to_string());
    }
}

// Manual impls of VisitBytes for VisitPrefixEncode to avoid conflict with blanket impls
impl VisitBytes for Checkpoint {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.visit_bv(visitor);
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimestampedCheckpoint {
    #[serde(flatten)]
    pub checkpoint: Checkpoint,
    pub timestamp: u64,
}

impl TimestampedCheckpoint {
    pub fn new(checkpoint: Checkpoint, time: SystemTime) -> anyhow::Result<Self> {
        Ok(Self {
            checkpoint,
            timestamp: time.duration_since(std::time::UNIX_EPOCH)?.as_secs(),
        })
    }

    pub fn now(checkpoint: Checkpoint) -> anyhow::Result<Self> {
        Self::new(checkpoint, SystemTime::now())
    }
}

impl Signable for TimestampedCheckpoint {
    const PREFIX: &'static [u8] = b"WARG-CHECKPOINT-SIGNATURE-V0";
}

impl prefix::VisitPrefixEncode for TimestampedCheckpoint {
    fn visit_pe<BV: ?Sized + ByteVisitor>(&self, visitor: &mut prefix::PrefixEncodeVisitor<BV>) {
        visitor.visit_str_raw("WARG-TIMESTAMPED-CHECKPOINT-V0");
        visitor.visit_unsigned(self.checkpoint.log_length as u64);
        visitor.visit_str(&self.checkpoint.log_root.to_string());
        visitor.visit_str(&self.checkpoint.map_root.to_string());
        visitor.visit_unsigned(self.timestamp);
    }
}

// Manual impls of VisitBytes for VisitPrefixEncode to avoid conflict with blanket impls
impl VisitBytes for TimestampedCheckpoint {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.visit_bv(visitor);
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MapLeaf {
    pub record_id: RecordId,
}

impl prefix::VisitPrefixEncode for MapLeaf {
    fn visit_pe<BV: ?Sized + ByteVisitor>(&self, visitor: &mut prefix::PrefixEncodeVisitor<BV>) {
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
#[serde(rename_all = "camelCase")]
pub struct LogLeaf {
    pub log_id: LogId,
    pub record_id: RecordId,
}

impl prefix::VisitPrefixEncode for LogLeaf {
    fn visit_pe<BV: ?Sized + ByteVisitor>(&self, visitor: &mut prefix::PrefixEncodeVisitor<BV>) {
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

/// Represents a valid package identifier in the registry.
///
/// Valid package identifiers conform to the component model specification
/// for identifiers.
///
/// A valid component model identifier is the format `<namespace>:<name>`,
/// where both parts are also valid WIT identifiers (i.e. kebab-cased).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct PackageId {
    id: String,
    colon: usize,
}

impl PackageId {
    /// Creates a package identifier from the given string.
    ///
    /// Returns an error if the given string is not a valid package identifier.
    pub fn new(id: impl Into<String>) -> anyhow::Result<Self> {
        let id = id.into();

        if let Some(colon) = id.rfind(':') {
            // Validate the namespace and name parts are valid kebab strings
            if KebabStr::new(&id[colon + 1..]).is_some() && Self::is_valid_namespace(&id[..colon]) {
                return Ok(Self { id, colon });
            }
        }

        bail!("invalid package identifier `{id}`: expected format is `<namespace>:<name>`")
    }

    /// Gets the namespace part of the package identifier.
    pub fn namespace(&self) -> &str {
        &self.id[..self.colon]
    }

    /// Gets the name part of the package identifier.
    pub fn name(&self) -> &str {
        &self.id[self.colon + 1..]
    }

    /// Check if string is a valid namespace.
    pub fn is_valid_namespace(namespace: &str) -> bool {
        const SUPPORTS_NESTED_NAMESPACES: bool = false;
        if SUPPORTS_NESTED_NAMESPACES {
            namespace.split(':').all(|s| KebabStr::new(s).is_some())
        } else {
            KebabStr::new(namespace).is_some()
        }
    }
}

impl AsRef<str> for PackageId {
    fn as_ref(&self) -> &str {
        &self.id
    }
}

impl FromStr for PackageId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{id}", id = self.id)
    }
}

impl prefix::VisitPrefixEncode for PackageId {
    fn visit_pe<BV: ?Sized + ByteVisitor>(&self, visitor: &mut prefix::PrefixEncodeVisitor<BV>) {
        visitor.visit_str_raw("WARG-PACKAGE-ID-V0");
        visitor.visit_str(&self.id);
    }
}

impl VisitBytes for PackageId {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.visit_bv(visitor);
    }
}

impl Serialize for PackageId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.id)
    }
}

impl<'de> Deserialize<'de> for PackageId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        PackageId::new(id).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LogId(AnyHash);

impl LogId {
    pub fn operator_log<D: SupportedDigest>() -> Self {
        let prefix: &[u8] = b"WARG-OPERATOR-LOG-ID-V0".as_slice();
        let hash: Hash<D> = Hash::of(prefix);
        Self(hash.into())
    }

    pub fn package_log<D: SupportedDigest>(id: &PackageId) -> Self {
        let prefix: &[u8] = b"WARG-PACKAGE-LOG-ID-V0:".as_slice();
        let hash: Hash<D> = Hash::of((prefix, id));
        Self(hash.into())
    }
}

impl fmt::Display for LogId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl VisitBytes for LogId {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        visitor.visit_bytes(self.0.bytes())
    }
}

impl From<AnyHash> for LogId {
    fn from(value: AnyHash) -> Self {
        Self(value)
    }
}

impl From<LogId> for AnyHash {
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
pub struct RecordId(AnyHash);

impl RecordId {
    pub fn algorithm(&self) -> HashAlgorithm {
        self.0.algorithm()
    }

    pub fn operator_record<D: SupportedDigest>(record: &ProtoEnvelope<OperatorRecord>) -> Self {
        let prefix: &[u8] = b"WARG-OPERATOR-LOG-RECORD-V0:".as_slice();
        let hash: Hash<D> = Hash::of((prefix, record.content_bytes()));
        Self(hash.into())
    }

    pub fn package_record<D: SupportedDigest>(record: &ProtoEnvelope<PackageRecord>) -> Self {
        let prefix: &[u8] = b"WARG-PACKAGE-LOG-RECORD-V0:".as_slice();
        let hash: Hash<D> = Hash::of((prefix, record.content_bytes()));
        Self(hash.into())
    }
}

impl fmt::Display for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<AnyHash> for RecordId {
    fn from(value: AnyHash) -> Self {
        Self(value)
    }
}

impl From<RecordId> for AnyHash {
    fn from(id: RecordId) -> Self {
        id.0
    }
}

impl AsRef<[u8]> for RecordId {
    fn as_ref(&self) -> &[u8] {
        self.0.bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warg_crypto::hash::Sha256;
    use warg_transparency::map::Map;

    #[test]
    fn log_id() {
        let first = Map::<Sha256, LogId, &'static str>::default();
        let second = first.insert(LogId::operator_log::<Sha256>(), "foobar");
        let proof = second.prove(LogId::operator_log::<Sha256>()).unwrap();
        assert_eq!(
            second.root().clone(),
            proof.evaluate(&LogId::operator_log::<Sha256>(), &"foobar")
        );
    }
}
