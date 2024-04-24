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

/// Represents a valid package name in the registry.
///
/// Valid package names conform to the component model specification.
///
/// A valid component model package name is the format `<namespace>:<name>`,
/// where both parts are also valid WIT identifiers (i.e. kebab-cased).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct PackageName {
    package_name: String,
    colon: usize,
}

impl PackageName {
    /// Creates a package name from the given string.
    ///
    /// Returns an error if the given string is not a valid package name.
    pub fn new(name: impl Into<String>) -> anyhow::Result<Self> {
        let name = name.into();

        if let Some(colon) = name.rfind(':') {
            // Validate the namespace and name parts are valid kebab strings
            if KebabStr::new(&name[colon + 1..]).is_some()
                && Self::is_valid_namespace(&name[..colon])
                && name[colon + 1..].chars().all(|c| !c.is_ascii_uppercase())
            {
                return Ok(Self {
                    package_name: name,
                    colon,
                });
            }
        }

        bail!("invalid package name `{name}`: expected format is `<namespace>:<name>` using lowercased characters")
    }

    /// Gets the namespace part of the package identifier.
    pub fn namespace(&self) -> &str {
        &self.package_name[..self.colon]
    }

    /// Gets the name part of the package identifier.
    pub fn name(&self) -> &str {
        &self.package_name[self.colon + 1..]
    }

    /// Check if string is a valid namespace.
    pub fn is_valid_namespace(namespace: &str) -> bool {
        KebabStr::new(namespace).is_some() && namespace.chars().all(|c| !c.is_ascii_uppercase())
    }
}

impl AsRef<str> for PackageName {
    fn as_ref(&self) -> &str {
        &self.package_name
    }
}

impl FromStr for PackageName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{package_name}", package_name = self.package_name)
    }
}

impl prefix::VisitPrefixEncode for PackageName {
    fn visit_pe<BV: ?Sized + ByteVisitor>(&self, visitor: &mut prefix::PrefixEncodeVisitor<BV>) {
        visitor.visit_str_raw("WARG-PACKAGE-ID-V0");
        visitor.visit_str(&self.package_name);
    }
}

impl VisitBytes for PackageName {
    fn visit<BV: ?Sized + ByteVisitor>(&self, visitor: &mut BV) {
        self.visit_bv(visitor);
    }
}

impl Serialize for PackageName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.package_name)
    }
}

impl<'de> Deserialize<'de> for PackageName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        PackageName::new(id).map_err(serde::de::Error::custom)
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

    pub fn package_log<D: SupportedDigest>(name: &PackageName) -> Self {
        let prefix: &[u8] = b"WARG-PACKAGE-LOG-ID-V0:".as_slice();
        let hash: Hash<D> = Hash::of((prefix, name));
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
