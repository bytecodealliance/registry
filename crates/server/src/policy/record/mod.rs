//! Module for server record policy implementations.
use thiserror::Error;
use warg_protocol::{package::PackageRecord, registry::PackageId, ProtoEnvelope};

mod authorization;
pub use authorization::*;

/// Represents a record policy error.
#[derive(Debug, Error)]
pub enum RecordPolicyError {
    /// A special rejection that indicates the record is not
    /// authorized to be published.
    ///
    /// Unauthorized records will never be stored.
    #[error("unauthorized operation:: {0}")]
    Unauthorized(String),
    /// The policy rejected the record with the given message.
    #[error("record was rejected by policy: {0}")]
    Rejection(String),
}

/// The result type returned by record policies.
pub type RecordPolicyResult<T> = Result<T, RecordPolicyError>;

/// A trait implemented by record policies.
pub trait RecordPolicy: Send + Sync {
    /// Checks the record against the policy.
    fn check(
        &self,
        id: &PackageId,
        record: &ProtoEnvelope<PackageRecord>,
    ) -> RecordPolicyResult<()>;
}

/// Represents a collection of record policies.
///
/// Record policies are checked in order of their addition
/// to the collection.
#[derive(Default)]
pub struct RecordPolicyCollection {
    policies: Vec<Box<dyn RecordPolicy>>,
}

impl RecordPolicyCollection {
    /// Creates a new record policy collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a new record policy into the collection.
    pub fn push(&mut self, policy: impl RecordPolicy + 'static) {
        self.policies.push(Box::new(policy));
    }
}

impl RecordPolicy for RecordPolicyCollection {
    fn check(
        &self,
        id: &PackageId,
        record: &ProtoEnvelope<PackageRecord>,
    ) -> RecordPolicyResult<()> {
        for policy in &self.policies {
            policy.check(id, record)?;
        }

        Ok(())
    }
}
