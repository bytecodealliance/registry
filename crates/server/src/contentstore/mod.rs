use thiserror::Error;
use tokio::fs::File;
use warg_crypto::hash::AnyHash;
use warg_protocol::registry::PackageId;

pub mod local;
pub mod oci;

#[derive(Debug, Error)]
pub enum ContentStoreError {
    #[error("content with address `{0}` was not found")]
    ContentNotFound(AnyHash),

    #[error("content store internal error: {0}")]
    ContentStoreInternalError(String),
}

/// Implemented by content stores.
#[axum::async_trait]
pub trait ContentStore: Send + Sync {
    /// Fetch content for a given package.
    async fn fetch_content(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
    ) -> Result<File, ContentStoreError>;

    /// Store content for a given package.
    async fn store_content(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
        content: &mut File,
    ) -> Result<String, ContentStoreError>;

    async fn content_present(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
    ) -> Result<bool, ContentStoreError>;
}
