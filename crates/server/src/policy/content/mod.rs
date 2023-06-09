//! Module for server content policy implementations.
use thiserror::Error;
use warg_crypto::hash::AnyHash;

mod wasm;

pub use wasm::*;

/// Represents a content policy error.
#[derive(Debug, Error)]
pub enum ContentPolicyError {
    /// The policy rejected the content with the given message.
    #[error("content was rejected by policy: {0}")]
    Rejection(String),
}

/// The result type returned by content policies.
pub type ContentPolicyResult<T> = Result<T, ContentPolicyError>;

/// A trait implemented by content policies.
pub trait ContentPolicy: Send + Sync {
    /// Creates a new stream policy for the given digest.
    ///
    /// The digest is provided so that a policy can make decisions
    /// based on the content's digest before any content is received.
    ///
    /// Upon success, returns a content stream policy that can be used
    /// to check the content as it is received.
    fn new_stream_policy(
        &self,
        digest: &AnyHash,
    ) -> ContentPolicyResult<Box<dyn ContentStreamPolicy>>;
}

/// A trait implemented by content stream policies.
pub trait ContentStreamPolicy: Send + Sync {
    /// Checks the given bytes of the content stream.
    ///
    /// The bytes represent the next chunk of data received for the
    /// content stream.
    fn check(&mut self, bytes: &[u8]) -> ContentPolicyResult<()>;

    /// Called when the content stream has finished.
    ///
    /// This method is called after all bytes have been received for
    /// the content stream.
    fn finalize(&mut self) -> ContentPolicyResult<()>;
}

/// Represents a collection of content policies.
///
/// Content policies are checked in order of their addition
/// to the collection.
#[derive(Default)]
pub struct ContentPolicyCollection {
    policies: Vec<Box<dyn ContentPolicy>>,
}

impl ContentPolicyCollection {
    /// Creates a new content policy collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a new content policy into the collection.
    pub fn push(&mut self, policy: impl ContentPolicy + 'static) {
        self.policies.push(Box::new(policy));
    }
}

impl ContentPolicy for ContentPolicyCollection {
    fn new_stream_policy(
        &self,
        digest: &AnyHash,
    ) -> ContentPolicyResult<Box<dyn ContentStreamPolicy>> {
        Ok(Box::new(ContentStreamPolicyCollection {
            policies: self
                .policies
                .iter()
                .map(|p| p.new_stream_policy(digest))
                .collect::<ContentPolicyResult<_>>()?,
        }))
    }
}

pub struct ContentStreamPolicyCollection {
    policies: Vec<Box<dyn ContentStreamPolicy>>,
}

impl ContentStreamPolicy for ContentStreamPolicyCollection {
    fn check(&mut self, bytes: &[u8]) -> ContentPolicyResult<()> {
        for policy in &mut self.policies {
            policy.check(bytes)?;
        }

        Ok(())
    }

    fn finalize(&mut self) -> ContentPolicyResult<()> {
        for policy in &mut self.policies {
            policy.finalize()?;
        }

        Ok(())
    }
}
