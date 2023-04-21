use futures::Stream;
use std::pin::Pin;
use thiserror::Error;
use warg_api::content::ContentSource;
use warg_crypto::hash::DynHash;
use warg_protocol::{
    operator, package,
    registry::{LogId, LogLeaf, MapCheckpoint, RecordId},
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
    CheckpointNotFound(DynHash),

    #[error("log `{0}` was not found")]
    LogNotFound(LogId),

    #[error("record `{0}` was not found")]
    RecordNotFound(RecordId),

    #[error("record contents for log `{record_id}` are invalid: {message}")]
    InvalidRecordContents {
        record_id: RecordId,
        message: String,
    },

    #[error("the operator record was invalid: {0}")]
    OperatorValidationFailed(#[from] operator::ValidationError),

    #[error("the package record was invalid: {0}")]
    PackageValidationFailed(#[from] package::ValidationError),

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
    pub leaf: LogLeaf,
    pub checkpoint: Option<DynHash>,
}

/// Represents the status of a record.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RecordStatus {
    Pending,
    Rejected(String),
    Accepted,
    InCheckpoint,
}

/// Represents an approved entry in an operator log.
pub struct OperatorLogEntry {
    /// The operator record.
    pub record: ProtoEnvelope<operator::OperatorRecord>,
    /// The checkpoint of the record.
    pub checkpoint: SerdeEnvelope<MapCheckpoint>,
}

/// Represents an approved entry in a package log.
#[derive(Debug, Clone)]
pub struct PackageLogEntry {
    /// The package record.
    pub record: ProtoEnvelope<package::PackageRecord>,
    /// The related sources.
    pub sources: Vec<ContentSource>,
    /// The checkpoint of the record.
    pub checkpoint: SerdeEnvelope<MapCheckpoint>,
}

/// Implemented by data stores.
#[axum::async_trait]
pub trait DataStore: Send + Sync {
    /// Iterate over the initial leaves in the store.
    ///
    /// This is an expensive operation and should only be performed on startup.
    async fn initial_leaves(
        &self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<InitialLeaf, DataStoreError>> + Send>>,
        DataStoreError,
    >;

    /// Stores the given operator record.
    ///
    /// Returns the record id of the new record.
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

    /// Accepts the given operator record.
    ///
    /// If the record fails to validate, it will be rejected.
    async fn accept_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<(), DataStoreError>;

    /// Stores the given package record.
    ///
    /// Returns the log leaf representing the new log record.
    async fn store_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        record: &ProtoEnvelope<package::PackageRecord>,
        sources: &[ContentSource],
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

    /// Accepts the given package record.
    ///
    /// If the record fails to validate, it will be rejected.
    async fn accept_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<(), DataStoreError>;

    /// Stores a new checkpoint.
    async fn store_checkpoint(
        &self,
        checkpoint_id: &DynHash,
        checkpoint: SerdeEnvelope<MapCheckpoint>,
        participants: &[LogLeaf],
    ) -> Result<(), DataStoreError>;

    /// Gets the latest checkpoint.
    async fn get_latest_checkpoint(&self) -> Result<SerdeEnvelope<MapCheckpoint>, DataStoreError>;

    /// Gets the operator records for the given registry root.
    async fn get_operator_records(
        &self,
        log_id: &LogId,
        root: &DynHash,
        since: Option<&RecordId>,
    ) -> Result<Vec<ProtoEnvelope<operator::OperatorRecord>>, DataStoreError>;

    /// Gets the package records for the given registry root.
    async fn get_package_records(
        &self,
        log_id: &LogId,
        root: &DynHash,
        since: Option<&RecordId>,
    ) -> Result<Vec<ProtoEnvelope<package::PackageRecord>>, DataStoreError>;

    /// Gets the status of a record.
    async fn get_record_status(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<RecordStatus, DataStoreError>;

    /// Gets an entry in an operator log.
    async fn get_operator_log_entry(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<OperatorLogEntry, DataStoreError>;

    /// Gets an entry in a package log.
    async fn get_package_log_entry(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<PackageLogEntry, DataStoreError>;
}
