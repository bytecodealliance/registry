use futures::Stream;
use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
};
use thiserror::Error;
use warg_crypto::{hash::AnyHash, signing::KeyID};
use warg_protocol::{
    operator, package,
    registry::{
        LogId, LogLeaf, PackageId, RecordId, RegistryIndex, RegistryLen, TimestampedCheckpoint,
    },
    ProtoEnvelope, PublishedProtoEnvelope, SerdeEnvelope,
};

mod memory;
#[cfg(feature = "postgres")]
mod postgres;

pub use memory::*;
#[cfg(feature = "postgres")]
pub use postgres::*;

#[derive(Debug, Error)]
pub enum DataStoreError {
    #[error("a conflicting operation was processed: update to the latest checkpoint and try the operation again")]
    Conflict,

    #[error("checkpoint log length `{0}` was not found")]
    CheckpointNotFound(RegistryLen),

    #[error("log `{0}` was not found")]
    LogNotFound(LogId),

    #[error("record `{0}` was not found")]
    RecordNotFound(RecordId),

    #[error("log leaf {0} was not found")]
    LogLeafNotFound(RegistryIndex),

    #[error("record `{0}` cannot be validated as it is not in a pending state")]
    RecordNotPending(RecordId),

    #[error("contents for record `{record_id}` are invalid: {message}")]
    InvalidRecordContents {
        record_id: RecordId,
        message: String,
    },

    #[error("the operator record was invalid: {0}")]
    OperatorValidationFailed(#[from] operator::ValidationError),

    #[error("the package record was invalid: {0}")]
    PackageValidationFailed(#[from] package::ValidationError),

    #[error("unknown key id `{0}`")]
    UnknownKey(KeyID),

    #[error("record signature verification failed")]
    SignatureVerificationFailed,

    #[error("the record was rejected: {0}")]
    Rejection(String),

    #[cfg(feature = "postgres")]
    #[error("a connection could not be established to the PostgreSQL server: {0}")]
    ConnectionPool(#[from] diesel_async::pooled_connection::deadpool::PoolError),

    #[cfg(feature = "postgres")]
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),
}

/// Represents the status of a record.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RecordStatus {
    /// The record is pending with missing content.
    MissingContent(Vec<AnyHash>),
    /// The record is pending with all content present.
    Pending,
    /// The record was rejected.
    Rejected(String),
    /// The record has been validated.
    Validated,
    /// The record was published (i.e. included in a registry checkpoint).
    Published,
}

/// Represents a record in a log.
pub struct Record<T>
where
    T: Clone,
{
    /// The status of the record.
    pub status: RecordStatus,
    /// The envelope containing the record contents.
    pub envelope: ProtoEnvelope<T>,
    /// The index of the record in the registry log.
    ///
    /// This is `None` if the record is not published.
    pub registry_index: Option<RegistryIndex>,
}

/// Implemented by data stores.
#[axum::async_trait]
pub trait DataStore: Send + Sync {
    /// Gets a stream of all checkpoints.
    ///
    /// This is an expensive operation and should only be performed on startup.
    async fn get_all_checkpoints(
        &self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<TimestampedCheckpoint, DataStoreError>> + Send>>,
        DataStoreError,
    >;

    /// Gets a stream of all validated records.
    ///
    /// This is an expensive operation and should only be performed on startup.
    async fn get_all_validated_records(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LogLeaf, DataStoreError>> + Send>>, DataStoreError>;

    /// Looks up the log_id and record_id from the registry log index.  
    async fn get_log_leafs_with_registry_index(
        &self,
        entries: &[RegistryIndex],
    ) -> Result<Vec<LogLeaf>, DataStoreError>;

    /// Stores the given operator record.
    async fn store_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        record: &ProtoEnvelope<operator::OperatorRecord>,
    ) -> Result<(), DataStoreError>;

    /// Rejects the given operator record.
    ///
    /// The record must be in the pending state.
    async fn reject_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        reason: &str,
    ) -> Result<(), DataStoreError>;

    /// Commits the given operator record.
    ///
    /// The record must be in a pending state.
    ///
    /// If validation succeeds, the record will be considered part of the log.
    async fn commit_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        registry_index: RegistryIndex,
    ) -> Result<(), DataStoreError>;

    /// Stores the given package record.
    ///
    /// The `missing` set is the set of content digests that are currently
    /// missing from data storage.
    async fn store_package_record(
        &self,
        log_id: &LogId,
        package_id: &PackageId,
        record_id: &RecordId,
        record: &ProtoEnvelope<package::PackageRecord>,
        missing: &HashSet<&AnyHash>,
    ) -> Result<(), DataStoreError>;

    /// Rejects the given package record.
    ///
    /// The record must be in the pending state.
    async fn reject_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        reason: &str,
    ) -> Result<(), DataStoreError>;

    /// Commits the given package record.
    ///
    /// The record must be in a pending state.
    ///
    /// If validation succeeds, the record will be considered part of the log.
    async fn commit_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        registry_index: RegistryIndex,
    ) -> Result<(), DataStoreError>;

    /// Determines if the given content digest is missing for the record.
    ///
    /// The record must be in a pending state.
    async fn is_content_missing(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        digest: &AnyHash,
    ) -> Result<bool, DataStoreError>;

    /// Sets the present flag for the given record and content digest.
    ///
    /// The record must be in a pending state.
    ///
    /// Returns true if the record has all of its content present as a
    /// result of this update.
    ///
    /// Returns false if the given digest was already marked present.
    async fn set_content_present(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        digest: &AnyHash,
    ) -> Result<bool, DataStoreError>;

    /// Stores a new checkpoint.
    async fn store_checkpoint(
        &self,
        checkpoint_id: &AnyHash,
        ts_checkpoint: SerdeEnvelope<TimestampedCheckpoint>,
    ) -> Result<(), DataStoreError>;

    /// Gets the latest checkpoint.
    async fn get_latest_checkpoint(
        &self,
    ) -> Result<SerdeEnvelope<TimestampedCheckpoint>, DataStoreError>;

    /// Get checkpoint by log length.
    async fn get_checkpoint(
        &self,
        log_length: RegistryLen,
    ) -> Result<SerdeEnvelope<TimestampedCheckpoint>, DataStoreError>;

    /// Gets package IDs from log IDs. If `PackageId` is unavailable, a corresponding `None` is returned.
    async fn get_package_ids(
        &self,
        log_ids: &[LogId],
    ) -> Result<HashMap<LogId, Option<PackageId>>, DataStoreError>;

    /// Gets a batch of log leafs starting with a registry log index.  
    async fn get_log_leafs_starting_with_registry_index(
        &self,
        starting_index: RegistryIndex,
        limit: Option<usize>,
    ) -> Result<Vec<(RegistryIndex, LogLeaf)>, DataStoreError>;

    /// Gets the operator records for the given registry log length.
    async fn get_operator_records(
        &self,
        log_id: &LogId,
        registry_log_length: RegistryLen,
        since: Option<&RecordId>,
        limit: u16,
    ) -> Result<Vec<PublishedProtoEnvelope<operator::OperatorRecord>>, DataStoreError>;

    /// Gets the package records for the given registry log length.
    async fn get_package_records(
        &self,
        log_id: &LogId,
        registry_log_length: RegistryLen,
        since: Option<&RecordId>,
        limit: u16,
    ) -> Result<Vec<PublishedProtoEnvelope<package::PackageRecord>>, DataStoreError>;

    /// Gets an operator record.
    async fn get_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<Record<operator::OperatorRecord>, DataStoreError>;

    /// Gets a package record.
    async fn get_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<Record<package::PackageRecord>, DataStoreError>;

    /// Verifies the signature of a package record.
    ///
    /// This is different from `validate_package_record` in that
    /// only the signature on the envelope is verified.
    ///
    /// It does not attempt to validate the record itself.
    async fn verify_package_record_signature(
        &self,
        log_id: &LogId,
        record: &ProtoEnvelope<package::PackageRecord>,
    ) -> Result<(), DataStoreError>;

    // Returns a list of package names, for debugging only.
    #[cfg(feature = "debug")]
    #[doc(hidden)]
    async fn debug_list_package_ids(&self) -> anyhow::Result<Vec<PackageId>> {
        anyhow::bail!("not implemented")
    }
}
