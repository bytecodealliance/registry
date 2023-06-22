use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::copy;
use warg_crypto::hash::AnyHash;
use warg_protocol::registry::PackageId;
use crate::contentstore::{ContentStore, ContentStoreError};

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
    async fn fetch_content(
        &self,
        _package_id: &PackageId,
        digest: &AnyHash,
    ) -> Result<File, ContentStoreError> {
        File::open(self.content_path(digest))
            .await
            .map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))
    }

    async fn store_content(
        &self,
        _package_id: &PackageId,
        digest: &AnyHash,
        content: &mut File
    ) -> Result<(), ContentStoreError> {
        let mut stored_file = File::create(self.content_path(digest))
            .await
            .map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))?;

        copy(content, &mut stored_file)
            .await
            .map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))?;
        Ok(())
    }

    async fn content_present(&self, _package_id: &PackageId, digest: &AnyHash) -> Result<bool, ContentStoreError> {
        let path = self.content_path(digest);
        Path::new(&path).try_exists().map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))
    }
}
