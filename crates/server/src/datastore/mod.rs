use futures::Stream;
use std::{collections::HashSet, pin::Pin};
use thiserror::Error;
use warg_crypto::{hash::AnyHash, signing::KeyID};
use warg_protocol::{
    operator, package,
    registry::{LogId, LogLeaf, MapCheckpoint, PackageId, RecordId},
    ProtoEnvelope, SerdeEnvelope,
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

    #[error("checkpoint `{0}` was not found")]
    CheckpointNotFound(AnyHash),

    #[error("log `{0}` was not found")]
    LogNotFound(LogId),

    #[error("record `{0}` was not found")]
    RecordNotFound(RecordId),

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

/// Represents a leaf used to initialize the core service.
pub struct InitialLeaf {
    /// The log leaf.
    pub leaf: LogLeaf,
    /// The checkpoint of the leaf.
    ///
    /// A value of `None` indicates the leaf needs to be included in the
    /// next checkpoint.
    pub checkpoint: Option<AnyHash>,
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
    /// The checkpoint of the record.
    ///
    /// This is `None` if the record is not published.
    pub checkpoint: Option<SerdeEnvelope<MapCheckpoint>>,
}

/// Implemented by data stores.
#[axum::async_trait]
pub trait DataStore: Send + Sync {
    /// Gets a stream of initial leaves in the store.
    ///
    /// This is an expensive operation and should only be performed on startup.
    async fn get_initial_leaves(
        &self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<InitialLeaf, DataStoreError>> + Send>>,
        DataStoreError,
    >;

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

    /// Validates the given operator record.
    ///
    /// The record must be in a pending state.
    ///
    /// If validation succeeds, the record will be considered part of the log.
    async fn validate_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
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

    /// Validates the given package record.
    ///
    /// The record must be in a pending state.
    ///
    /// If validation succeeds, the record will be considered part of the log.
    async fn validate_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
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
        checkpoint: SerdeEnvelope<MapCheckpoint>,
        participants: &[LogLeaf],
    ) -> Result<(), DataStoreError>;

    /// Gets the latest checkpoint.
    async fn get_latest_checkpoint(&self) -> Result<SerdeEnvelope<MapCheckpoint>, DataStoreError>;

    /// Gets the operator records for the given registry root.
    async fn get_operator_records(
        &self,
        log_id: &LogId,
        root: &AnyHash,
        since: Option<&RecordId>,
        limit: u16,
    ) -> Result<Vec<ProtoEnvelope<operator::OperatorRecord>>, DataStoreError>;

    /// Gets the package records for the given registry root.
    async fn get_package_records(
        &self,
        log_id: &LogId,
        root: &AnyHash,
        since: Option<&RecordId>,
        limit: u16,
    ) -> Result<Vec<ProtoEnvelope<package::PackageRecord>>, DataStoreError>;

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
}
