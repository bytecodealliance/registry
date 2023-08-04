use hyper::Uri;
use thiserror::Error;
use tokio::fs::File;
use warg_crypto::hash::AnyHash;
use warg_protocol::registry::PackageId;

pub mod local;
#[cfg(feature = "oci")]
pub mod oci;
#[cfg(feature = "s3")]
pub mod s3;

#[derive(Debug, Error)]
pub enum ContentStoreError {
    #[error("content with address `{0}` was not found")]
    ContentNotFound(AnyHash),

    #[error("content store internal error: {0}")]
    ContentStoreInternalError(String),
}

pub enum ContentStoreUriSigning {
    None,
    Presigned(Box<dyn PresignedContentStore>),
}

/// Implemented by content stores that support presigned URIs.
#[axum::async_trait]
pub trait PresignedContentStore {
    async fn read_uri(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
    ) -> Result<Uri, ContentStoreError>;
    async fn write_uri(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
    ) -> Result<Uri, ContentStoreError>;
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

    async fn uri_signing(&self) -> ContentStoreUriSigning {
        ContentStoreUriSigning::None
    }
}
