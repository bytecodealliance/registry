use crate::contentstore::{ContentStore, ContentStoreError};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::copy;
use warg_crypto::hash::AnyHash;
use warg_protocol::registry::PackageId;

#[derive(Clone)]
pub struct LocalContentStore {
    files_dir: PathBuf,
}

impl LocalContentStore {
    pub fn new(files_dir: PathBuf) -> Self {
        Self { files_dir }
    }

    /// Returns the path to the content file for a given content address.
    fn content_path(&self, digest: &AnyHash) -> PathBuf {
        self.files_dir.join(content_file_name(digest))
    }
}

/// Returns the file name for a given content address replacing colons with dashes.
fn content_file_name(digest: &AnyHash) -> String {
    digest.to_string().replace(':', "-")
}

#[axum::async_trait]
impl ContentStore for LocalContentStore {
    /// Fetch content for a given package.
    async fn fetch_content(
        &self,
        _package_id: &PackageId,
        digest: &AnyHash,
        _version: String,
    ) -> Result<File, ContentStoreError> {
        File::open(self.content_path(digest))
            .await
            .map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))
    }

    /// Store content for a given package.
    async fn store_content(
        &self,
        _package_id: &PackageId,
        digest: &AnyHash,
        _version: String,
        content: &mut File,
    ) -> Result<String, ContentStoreError> {
        let file_path = self.content_path(digest);
        let mut stored_file = File::create(file_path.clone())
            .await
            .map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))?;

        copy(content, &mut stored_file)
            .await
            .map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))?;
        Ok(file_path.to_string_lossy().to_string())
    }

    /// Check if the content is present in the store.
    async fn content_present(
        &self,
        _package_id: &PackageId,
        digest: &AnyHash,
        _version: String,
    ) -> Result<bool, ContentStoreError> {
        let path = self.content_path(digest);
        Path::new(&path)
            .try_exists()
            .map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))
    }
}
