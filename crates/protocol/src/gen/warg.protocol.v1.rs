#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Envelope {
    #[prost(bytes = "vec", tag = "1")]
    pub contents: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "2")]
    pub key_id: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub signature: ::prost::alloc::string::String,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OperatorRecord {
    /// The previous entry in the log.
    /// First entry of a log has no previous entry.
    #[prost(string, optional, tag = "1")]
    pub prev: ::core::option::Option<::prost::alloc::string::String>,
    /// The warg protocol version.
    #[prost(uint32, tag = "2")]
    pub version: u32,
    /// The time when this entry was created
    #[prost(message, optional, tag = "3")]
    pub time: ::core::option::Option<::prost_wkt_types::Timestamp>,
    /// The content specific to this entry type
    #[prost(message, repeated, tag = "4")]
    pub entries: ::prost::alloc::vec::Vec<OperatorEntry>,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OperatorEntry {
    #[prost(oneof = "operator_entry::Contents", tags = "1, 2, 3")]
    pub contents: ::core::option::Option<operator_entry::Contents>,
}
/// Nested message and enum types in `OperatorEntry`.
pub mod operator_entry {
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Contents {
        #[prost(message, tag = "1")]
        Init(super::OperatorInit),
        #[prost(message, tag = "2")]
        GrantFlat(super::OperatorGrantFlat),
        #[prost(message, tag = "3")]
        RevokeFlat(super::OperatorRevokeFlat),
    }
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OperatorInit {
    /// The hash algorithm used by this package to link entries.
    #[prost(string, tag = "1")]
    pub hash_algorithm: ::prost::alloc::string::String,
    /// The key for the author of this entry.
    #[prost(string, tag = "2")]
    pub key: ::prost::alloc::string::String,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OperatorGrantFlat {
    /// The key being given the permission.
    #[prost(string, tag = "1")]
    pub key: ::prost::alloc::string::String,
    /// The permission to grant the key.
    #[prost(enumeration = "OperatorPermission", tag = "2")]
    pub permission: i32,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OperatorRevokeFlat {
    /// The key whose permission is being revoked.
    #[prost(string, tag = "1")]
    pub key_id: ::prost::alloc::string::String,
    /// The permission to grant the key.
    #[prost(enumeration = "OperatorPermission", tag = "2")]
    pub permission: i32,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackageRecord {
    /// The previous entry in the log.
    /// First entry of a log has no previous entry.
    #[prost(string, optional, tag = "1")]
    pub prev: ::core::option::Option<::prost::alloc::string::String>,
    /// The warg protocol version.
    #[prost(uint32, tag = "2")]
    pub version: u32,
    /// The time when this entry was created
    #[prost(message, optional, tag = "3")]
    pub time: ::core::option::Option<::prost_wkt_types::Timestamp>,
    #[prost(message, repeated, tag = "4")]
    pub entries: ::prost::alloc::vec::Vec<PackageEntry>,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackageEntry {
    #[prost(oneof = "package_entry::Contents", tags = "1, 2, 3, 4, 5")]
    pub contents: ::core::option::Option<package_entry::Contents>,
}
/// Nested message and enum types in `PackageEntry`.
pub mod package_entry {
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Contents {
        #[prost(message, tag = "1")]
        Init(super::PackageInit),
        #[prost(message, tag = "2")]
        GrantFlat(super::PackageGrantFlat),
        #[prost(message, tag = "3")]
        RevokeFlat(super::PackageRevokeFlat),
        #[prost(message, tag = "4")]
        Release(super::PackageRelease),
        #[prost(message, tag = "5")]
        Yank(super::PackageYank),
    }
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackageInit {
    #[prost(string, tag = "1")]
    pub key: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub hash_algorithm: ::prost::alloc::string::String,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackageGrantFlat {
    #[prost(string, tag = "1")]
    pub key: ::prost::alloc::string::String,
    #[prost(enumeration = "PackagePermission", tag = "2")]
    pub permission: i32,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackageRevokeFlat {
    #[prost(string, tag = "1")]
    pub key_id: ::prost::alloc::string::String,
    #[prost(enumeration = "PackagePermission", tag = "2")]
    pub permission: i32,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackageRelease {
    #[prost(string, tag = "1")]
    pub version: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub content_hash: ::prost::alloc::string::String,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackageYank {
    #[prost(string, tag = "1")]
    pub version: ::prost::alloc::string::String,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ContentSource {
    #[prost(message, optional, tag = "1")]
    pub digest: ::core::option::Option<DynHash>,
    #[prost(message, optional, tag = "2")]
    pub kind: ::core::option::Option<ContentSourceKind>,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DynHash {
    #[prost(enumeration = "HashAlgorithm", tag = "1")]
    pub algo: i32,
    #[prost(bytes = "vec", tag = "2")]
    pub bytes: ::prost::alloc::vec::Vec<u8>,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ContentSourceKind {
    #[prost(oneof = "content_source_kind::Kind", tags = "1")]
    pub kind: ::core::option::Option<content_source_kind::Kind>,
}
/// Nested message and enum types in `ContentSourceKind`.
pub mod content_source_kind {
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        HttpAnonymous(super::HttpAnonymousContentSource),
    }
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HttpAnonymousContentSource {
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackageRecordId {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub record_id: ::core::option::Option<DynHash>,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MapCheckpoint {
    #[prost(message, optional, tag = "1")]
    pub log_root: ::core::option::Option<DynHash>,
    #[prost(uint32, tag = "2")]
    pub log_length: u32,
    #[prost(message, optional, tag = "3")]
    pub map_root: ::core::option::Option<DynHash>,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LogLeaf {
    #[prost(message, optional, tag = "1")]
    pub log_id: ::core::option::Option<DynHash>,
    #[prost(message, optional, tag = "2")]
    pub record_id: ::core::option::Option<DynHash>,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum OperatorPermission {
    Unspecified = 0,
    Commit = 1,
}
impl OperatorPermission {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            OperatorPermission::Unspecified => "OPERATOR_PERMISSION_UNSPECIFIED",
            OperatorPermission::Commit => "OPERATOR_PERMISSION_COMMIT",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "OPERATOR_PERMISSION_UNSPECIFIED" => Some(Self::Unspecified),
            "OPERATOR_PERMISSION_COMMIT" => Some(Self::Commit),
            _ => None,
        }
    }
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum PackagePermission {
    Unspecified = 0,
    Release = 1,
    Yank = 2,
}
impl PackagePermission {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            PackagePermission::Unspecified => "PACKAGE_PERMISSION_UNSPECIFIED",
            PackagePermission::Release => "PACKAGE_PERMISSION_RELEASE",
            PackagePermission::Yank => "PACKAGE_PERMISSION_YANK",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "PACKAGE_PERMISSION_UNSPECIFIED" => Some(Self::Unspecified),
            "PACKAGE_PERMISSION_RELEASE" => Some(Self::Release),
            "PACKAGE_PERMISSION_YANK" => Some(Self::Yank),
            _ => None,
        }
    }
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum HashAlgorithm {
    Unknown = 0,
    Sha256 = 1,
}
impl HashAlgorithm {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            HashAlgorithm::Unknown => "HASH_ALGORITHM_UNKNOWN",
            HashAlgorithm::Sha256 => "HASH_ALGORITHM_SHA256",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "HASH_ALGORITHM_UNKNOWN" => Some(Self::Unknown),
            "HASH_ALGORITHM_SHA256" => Some(Self::Sha256),
            _ => None,
        }
    }
}
/// PublishPackageRequest summary...
///
/// PublishPackageRequest description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublishPackageRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub record: ::core::option::Option<Envelope>,
    #[prost(message, repeated, tag = "3")]
    pub content_sources: ::prost::alloc::vec::Vec<ContentSource>,
}
/// PublishPackageResponse summary...
///
/// PublishPackageResponse description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PublishPackageResponse {
    #[prost(message, optional, tag = "1")]
    pub package: ::core::option::Option<Package>,
}
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPackageRequest {
    /// IDEA: Could add field mask to return more details like records.
    #[prost(string, tag = "1")]
    pub package_id: ::prost::alloc::string::String,
}
/// GetPackageResponse summary...
///
/// GetPackageResponse description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPackageResponse {
    #[prost(message, optional, tag = "1")]
    pub package: ::core::option::Option<Package>,
}
/// Package summary...
///
/// NOTE: Replaces PendingRecordResponse from axios API
/// NOTE: Records could optionally be added if field mask added to get API call
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Package {
    #[prost(string, tag = "1")]
    pub package_id: ::prost::alloc::string::String,
    #[prost(enumeration = "PackageStatusCode", tag = "2")]
    pub status_code: i32,
    #[prost(string, tag = "3")]
    pub status_message: ::prost::alloc::string::String,
}
/// GetPackageRecordRequest summary...
///
/// GetPackageRecordRequest description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPackageRecordRequest {
    #[prost(string, tag = "1")]
    pub package_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub record_id: ::prost::alloc::string::String,
}
/// Record summary...
///
/// QUESTION: Why axios structure different than PackageRecord message?
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Record {
    #[prost(string, tag = "1")]
    pub package_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub record_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "3")]
    pub record: ::core::option::Option<Envelope>,
    #[prost(message, repeated, tag = "4")]
    pub content_sources: ::prost::alloc::vec::Vec<ContentSource>,
}
/// FetchLogsRequest summary...
///
/// FetchLogsRequest description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FetchLogsRequest {
    #[prost(message, optional, tag = "1")]
    pub root: ::core::option::Option<DynHash>,
    /// Operator RecordId
    #[prost(message, optional, tag = "2")]
    pub operator: ::core::option::Option<DynHash>,
    /// Ordered by iteration order of IndexMap\<String, Option<RecordId>\>
    /// QUESTION: How is generic client to know iteration order?
    #[prost(message, repeated, tag = "3")]
    pub packages: ::prost::alloc::vec::Vec<PackageRecordId>,
}
/// FetchLogsResponse summary...
///
/// FetchLogsResponse description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FetchLogsResponse {
    #[prost(message, repeated, tag = "1")]
    pub operator_records: ::prost::alloc::vec::Vec<Envelope>,
    /// Ordered by iteration order of IndexMap\<String, Option<RecordId>\>
    #[prost(message, repeated, tag = "2")]
    pub packages: ::prost::alloc::vec::Vec<PackageRecordId>,
}
/// Requests latest checkpoint if no fields are set.
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FetchCheckpointRequest {}
/// FetchCheckpointResponse summary...
///
/// FetchCheckpointResponse description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FetchCheckpointResponse {
    #[prost(message, optional, tag = "1")]
    pub checkpoint: ::core::option::Option<MapCheckpoint>,
}
/// ProveConsistencyRequest summary...
///
/// ProveConsistencyRequest description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProveConsistencyRequest {
    /// The old root to check for consistency.
    #[prost(message, optional, tag = "1")]
    pub old_root: ::core::option::Option<DynHash>,
    /// The new root to check for consistency.
    #[prost(message, optional, tag = "2")]
    pub new_root: ::core::option::Option<DynHash>,
}
/// ProveConsistencyResponse summary...
///
/// ProveConsistencyResponse description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProveConsistencyResponse {
    /// The kind of log to prove consistency.
    #[prost(oneof = "prove_consistency_response::LogKind", tags = "1")]
    pub log_kind: ::core::option::Option<prove_consistency_response::LogKind>,
}
/// Nested message and enum types in `ProveConsistencyResponse`.
pub mod prove_consistency_response {
    /// The kind of log to prove consistency.
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum LogKind {
        /// TODO: Create formal definition of proof in cross-platform proto
        #[prost(bytes, tag = "1")]
        EncodedLogBundle(::prost::alloc::vec::Vec<u8>),
    }
}
/// ProveInclusionRequest summary...
///
/// ProveInclusionRequest description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProveInclusionRequest {
    #[prost(message, optional, tag = "1")]
    pub checkpoint: ::core::option::Option<MapCheckpoint>,
    #[prost(message, repeated, tag = "2")]
    pub heads: ::prost::alloc::vec::Vec<LogLeaf>,
}
/// ProveInclusionResponse summary...
///
/// ProveInclusionResponse description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProveInclusionResponse {
    /// The kind of log to prove inclusion.
    #[prost(oneof = "prove_inclusion_response::LogKind", tags = "1")]
    pub log_kind: ::core::option::Option<prove_inclusion_response::LogKind>,
    /// The kind of map to prove inclusion.
    #[prost(oneof = "prove_inclusion_response::MapKind", tags = "2")]
    pub map_kind: ::core::option::Option<prove_inclusion_response::MapKind>,
}
/// Nested message and enum types in `ProveInclusionResponse`.
pub mod prove_inclusion_response {
    /// The kind of log to prove inclusion.
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum LogKind {
        /// TODO: Create formal definition of proof in cross-platform proto
        #[prost(bytes, tag = "1")]
        EncodedLogBundle(::prost::alloc::vec::Vec<u8>),
    }
    /// The kind of map to prove inclusion.
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum MapKind {
        /// TODO: Create formal definition of proof in cross-platform proto
        #[prost(bytes, tag = "2")]
        EncodedMapBundle(::prost::alloc::vec::Vec<u8>),
    }
}
/// ContentFetchFailure summary...
///
/// ContentFetchFailure description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ContentFetchFailure {
    #[prost(oneof = "content_fetch_failure::Kind", tags = "1")]
    pub kind: ::core::option::Option<content_fetch_failure::Kind>,
}
/// Nested message and enum types in `ContentFetchFailure`.
pub mod content_fetch_failure {
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Kind {
        /// Only status (code) is guaranteed to be set. Others may be set based on
        /// server-configured options.
        #[prost(message, tag = "1")]
        HttpResponse(crate::google_pb::HttpResponse),
    }
}
/// PackageStatusCode summary...
///
/// PackageStatusCode description...
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum PackageStatusCode {
    /// Used when package status is unknown
    Unknown = 0,
    /// Used when package publish is still pending.
    Pending = 1,
    /// Used when package is published and active.
    Published = 2,
}
impl PackageStatusCode {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            PackageStatusCode::Unknown => "PACKAGE_STATUS_CODE_UNKNOWN",
            PackageStatusCode::Pending => "PACKAGE_STATUS_CODE_PENDING",
            PackageStatusCode::Published => "PACKAGE_STATUS_CODE_PUBLISHED",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "PACKAGE_STATUS_CODE_UNKNOWN" => Some(Self::Unknown),
            "PACKAGE_STATUS_CODE_PENDING" => Some(Self::Pending),
            "PACKAGE_STATUS_CODE_PUBLISHED" => Some(Self::Published),
            _ => None,
        }
    }
}
/// Errors are defined in Rust and are mapped to google.rpc.ErrorInfo with
/// domain github.com/bytecodealliance/registry.
///
/// The reasons and key names are
/// defined by the following enums. Custom error message types should be created
/// when a client should programmatically attempt to retry with certain details.
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ErrorReason {
    Unknown = 0,
    General = 1,
    InvalidPackageId = 3,
    InvalidRecordId = 4,
    InvalidRecord = 5,
    PackageIdNotFound = 6,
    PackageNotFound = 7,
    PackageRecordNotFound = 8,
    FailedToFetchContent = 9,
    ContentUrlInvalid = 10,
    OperationInvocationFailed = 11,
}
impl ErrorReason {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ErrorReason::Unknown => "ERROR_REASON_UNKNOWN",
            ErrorReason::General => "ERROR_REASON_GENERAL",
            ErrorReason::InvalidPackageId => "ERROR_REASON_INVALID_PACKAGE_ID",
            ErrorReason::InvalidRecordId => "ERROR_REASON_INVALID_RECORD_ID",
            ErrorReason::InvalidRecord => "ERROR_REASON_INVALID_RECORD",
            ErrorReason::PackageIdNotFound => "ERROR_REASON_PACKAGE_ID_NOT_FOUND",
            ErrorReason::PackageNotFound => "ERROR_REASON_PACKAGE_NOT_FOUND",
            ErrorReason::PackageRecordNotFound => "ERROR_REASON_PACKAGE_RECORD_NOT_FOUND",
            ErrorReason::FailedToFetchContent => "ERROR_REASON_FAILED_TO_FETCH_CONTENT",
            ErrorReason::ContentUrlInvalid => "ERROR_REASON_CONTENT_URL_INVALID",
            ErrorReason::OperationInvocationFailed => {
                "ERROR_REASON_OPERATION_INVOCATION_FAILED"
            }
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "ERROR_REASON_UNKNOWN" => Some(Self::Unknown),
            "ERROR_REASON_GENERAL" => Some(Self::General),
            "ERROR_REASON_INVALID_PACKAGE_ID" => Some(Self::InvalidPackageId),
            "ERROR_REASON_INVALID_RECORD_ID" => Some(Self::InvalidRecordId),
            "ERROR_REASON_INVALID_RECORD" => Some(Self::InvalidRecord),
            "ERROR_REASON_PACKAGE_ID_NOT_FOUND" => Some(Self::PackageIdNotFound),
            "ERROR_REASON_PACKAGE_NOT_FOUND" => Some(Self::PackageNotFound),
            "ERROR_REASON_PACKAGE_RECORD_NOT_FOUND" => Some(Self::PackageRecordNotFound),
            "ERROR_REASON_FAILED_TO_FETCH_CONTENT" => Some(Self::FailedToFetchContent),
            "ERROR_REASON_CONTENT_URL_INVALID" => Some(Self::ContentUrlInvalid),
            "ERROR_REASON_OPERATION_INVOCATION_FAILED" => {
                Some(Self::OperationInvocationFailed)
            }
            _ => None,
        }
    }
}
/// Generated client implementations.
pub mod warg_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Errors are returned use the standard mechanism in gRPC: google.rpc.Status.
    /// When available its details will contain an google.rpc.ErrorInfo with domain
    /// "github.com/bytecodealliance/registry" and reason defined by the keys in
    /// ErrorReason.
    ///
    /// NOTE: Follows current axios API as closely as possible, but makes some style
    /// changes to reasonably fit proto/api style guide.
    #[derive(Debug, Clone)]
    pub struct WargClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl WargClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> WargClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> WargClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            WargClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Request that a new package be published.
        ///
        /// NOTE: Current axios API has PublishRequest => PendingRecordResponse
        pub async fn publish_package(
            &mut self,
            request: impl tonic::IntoRequest<super::PublishPackageRequest>,
        ) -> std::result::Result<
            tonic::Response<super::PublishPackageResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/warg.protocol.v1.Warg/PublishPackage",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("warg.protocol.v1.Warg", "PublishPackage"));
            self.inner.unary(req, path, codec).await
        }
        /// Used for polling while package is being in the processed of publishing.
        ///
        /// NOTE: This is a substitute for /package/{package_id}/pending/{record_id}
        /// which seemed superfluous.
        pub async fn get_package(
            &mut self,
            request: impl tonic::IntoRequest<super::GetPackageRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetPackageResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/warg.protocol.v1.Warg/GetPackage",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("warg.protocol.v1.Warg", "GetPackage"));
            self.inner.unary(req, path, codec).await
        }
        /// Get a specific record within a package.
        pub async fn get_package_record(
            &mut self,
            request: impl tonic::IntoRequest<super::GetPackageRecordRequest>,
        ) -> std::result::Result<tonic::Response<super::Record>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/warg.protocol.v1.Warg/GetPackageRecord",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("warg.protocol.v1.Warg", "GetPackageRecord"));
            self.inner.unary(req, path, codec).await
        }
        /// Fetches logs for a requested package.
        ///
        /// NOTE: Current axios API uses /fetch/logs
        pub async fn fetch_logs(
            &mut self,
            request: impl tonic::IntoRequest<super::FetchLogsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::FetchLogsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/warg.protocol.v1.Warg/FetchLogs",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("warg.protocol.v1.Warg", "FetchLogs"));
            self.inner.unary(req, path, codec).await
        }
        /// Fetches logs for a root.
        ///
        /// NOTE: Current axios API uses /fetch/checkpoint
        pub async fn fetch_checkpoint(
            &mut self,
            request: impl tonic::IntoRequest<super::FetchCheckpointRequest>,
        ) -> std::result::Result<
            tonic::Response<super::FetchCheckpointResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/warg.protocol.v1.Warg/FetchCheckpoint",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("warg.protocol.v1.Warg", "FetchCheckpoint"));
            self.inner.unary(req, path, codec).await
        }
        /// Proves consistency between an old root and a new one.
        ///
        /// NOTE: Current axios API uses /proof/consistency
        pub async fn prove_consistency(
            &mut self,
            request: impl tonic::IntoRequest<super::ProveConsistencyRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ProveConsistencyResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/warg.protocol.v1.Warg/ProveConsistency",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("warg.protocol.v1.Warg", "ProveConsistency"));
            self.inner.unary(req, path, codec).await
        }
        /// Proves inclusion between a log and a map.
        ///
        /// NOTE: Current axios API uses /proof/inclusion
        pub async fn prove_inclusion(
            &mut self,
            request: impl tonic::IntoRequest<super::ProveInclusionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ProveInclusionResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/warg.protocol.v1.Warg/ProveInclusion",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("warg.protocol.v1.Warg", "ProveInclusion"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod warg_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with WargServer.
    #[async_trait]
    pub trait Warg: Send + Sync + 'static {
        /// Request that a new package be published.
        ///
        /// NOTE: Current axios API has PublishRequest => PendingRecordResponse
        async fn publish_package(
            &self,
            request: tonic::Request<super::PublishPackageRequest>,
        ) -> std::result::Result<
            tonic::Response<super::PublishPackageResponse>,
            tonic::Status,
        >;
        /// Used for polling while package is being in the processed of publishing.
        ///
        /// NOTE: This is a substitute for /package/{package_id}/pending/{record_id}
        /// which seemed superfluous.
        async fn get_package(
            &self,
            request: tonic::Request<super::GetPackageRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetPackageResponse>,
            tonic::Status,
        >;
        /// Get a specific record within a package.
        async fn get_package_record(
            &self,
            request: tonic::Request<super::GetPackageRecordRequest>,
        ) -> std::result::Result<tonic::Response<super::Record>, tonic::Status>;
        /// Fetches logs for a requested package.
        ///
        /// NOTE: Current axios API uses /fetch/logs
        async fn fetch_logs(
            &self,
            request: tonic::Request<super::FetchLogsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::FetchLogsResponse>,
            tonic::Status,
        >;
        /// Fetches logs for a root.
        ///
        /// NOTE: Current axios API uses /fetch/checkpoint
        async fn fetch_checkpoint(
            &self,
            request: tonic::Request<super::FetchCheckpointRequest>,
        ) -> std::result::Result<
            tonic::Response<super::FetchCheckpointResponse>,
            tonic::Status,
        >;
        /// Proves consistency between an old root and a new one.
        ///
        /// NOTE: Current axios API uses /proof/consistency
        async fn prove_consistency(
            &self,
            request: tonic::Request<super::ProveConsistencyRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ProveConsistencyResponse>,
            tonic::Status,
        >;
        /// Proves inclusion between a log and a map.
        ///
        /// NOTE: Current axios API uses /proof/inclusion
        async fn prove_inclusion(
            &self,
            request: tonic::Request<super::ProveInclusionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ProveInclusionResponse>,
            tonic::Status,
        >;
    }
    /// Errors are returned use the standard mechanism in gRPC: google.rpc.Status.
    /// When available its details will contain an google.rpc.ErrorInfo with domain
    /// "github.com/bytecodealliance/registry" and reason defined by the keys in
    /// ErrorReason.
    ///
    /// NOTE: Follows current axios API as closely as possible, but makes some style
    /// changes to reasonably fit proto/api style guide.
    #[derive(Debug)]
    pub struct WargServer<T: Warg> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Warg> WargServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for WargServer<T>
    where
        T: Warg,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/warg.protocol.v1.Warg/PublishPackage" => {
                    #[allow(non_camel_case_types)]
                    struct PublishPackageSvc<T: Warg>(pub Arc<T>);
                    impl<
                        T: Warg,
                    > tonic::server::UnaryService<super::PublishPackageRequest>
                    for PublishPackageSvc<T> {
                        type Response = super::PublishPackageResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PublishPackageRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).publish_package(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PublishPackageSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/warg.protocol.v1.Warg/GetPackage" => {
                    #[allow(non_camel_case_types)]
                    struct GetPackageSvc<T: Warg>(pub Arc<T>);
                    impl<T: Warg> tonic::server::UnaryService<super::GetPackageRequest>
                    for GetPackageSvc<T> {
                        type Response = super::GetPackageResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetPackageRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move { (*inner).get_package(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetPackageSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/warg.protocol.v1.Warg/GetPackageRecord" => {
                    #[allow(non_camel_case_types)]
                    struct GetPackageRecordSvc<T: Warg>(pub Arc<T>);
                    impl<
                        T: Warg,
                    > tonic::server::UnaryService<super::GetPackageRecordRequest>
                    for GetPackageRecordSvc<T> {
                        type Response = super::Record;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetPackageRecordRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_package_record(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetPackageRecordSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/warg.protocol.v1.Warg/FetchLogs" => {
                    #[allow(non_camel_case_types)]
                    struct FetchLogsSvc<T: Warg>(pub Arc<T>);
                    impl<T: Warg> tonic::server::UnaryService<super::FetchLogsRequest>
                    for FetchLogsSvc<T> {
                        type Response = super::FetchLogsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FetchLogsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move { (*inner).fetch_logs(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FetchLogsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/warg.protocol.v1.Warg/FetchCheckpoint" => {
                    #[allow(non_camel_case_types)]
                    struct FetchCheckpointSvc<T: Warg>(pub Arc<T>);
                    impl<
                        T: Warg,
                    > tonic::server::UnaryService<super::FetchCheckpointRequest>
                    for FetchCheckpointSvc<T> {
                        type Response = super::FetchCheckpointResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FetchCheckpointRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).fetch_checkpoint(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FetchCheckpointSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/warg.protocol.v1.Warg/ProveConsistency" => {
                    #[allow(non_camel_case_types)]
                    struct ProveConsistencySvc<T: Warg>(pub Arc<T>);
                    impl<
                        T: Warg,
                    > tonic::server::UnaryService<super::ProveConsistencyRequest>
                    for ProveConsistencySvc<T> {
                        type Response = super::ProveConsistencyResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ProveConsistencyRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).prove_consistency(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ProveConsistencySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/warg.protocol.v1.Warg/ProveInclusion" => {
                    #[allow(non_camel_case_types)]
                    struct ProveInclusionSvc<T: Warg>(pub Arc<T>);
                    impl<
                        T: Warg,
                    > tonic::server::UnaryService<super::ProveInclusionRequest>
                    for ProveInclusionSvc<T> {
                        type Response = super::ProveInclusionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ProveInclusionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).prove_inclusion(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ProveInclusionSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Warg> Clone for WargServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: Warg> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Warg> tonic::server::NamedService for WargServer<T> {
        const NAME: &'static str = "warg.protocol.v1.Warg";
    }
}
