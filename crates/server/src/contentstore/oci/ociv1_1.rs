use std::path::PathBuf;
use oci_distribution::secrets::RegistryAuth;
use tokio::fs::File;
use warg_crypto::hash::AnyHash;
use warg_protocol::registry::PackageId;
use crate::contentstore::{ContentStore, ContentStoreError};
use crate::contentstore::oci::client::Client;

type Auth = RegistryAuth;

/// Content store for OCI v1.1 registries.
pub struct OCIv1_1ContentStore {
    client: Client,
    registry_url: String,
}

impl OCIv1_1ContentStore {
    pub async fn new(registry_url: impl Into<String>, auth: Auth, temp_dir: &PathBuf) -> Self {
        let client = Client::new(true, auth, temp_dir).await;
        Self { client, registry_url: registry_url.into() }
    }

    fn reference(&self, package_id: &PackageId, version: String) -> String {
        let (reg_url, namespace, name) = (self.registry_url.clone(), package_id.namespace(), package_id.name());
        format!("{reg_url}/{namespace}/{name}:{version}")
    }
}

#[axum::async_trait]
impl ContentStore for OCIv1_1ContentStore {
    /// Fetch the content from the store.
    async fn fetch_content(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
    ) -> Result<File, ContentStoreError> {
        let reference = self.reference(package_id, version);
        self
            .client
            .pull(reference, digest)
            .await
    }

    /// Store the content in the store.
    async fn store_content(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
        content: &mut File
    ) -> Result<String, ContentStoreError> {
        let reference = self.reference(package_id, version);
        self
            .client
            .push(reference, content, digest)
            .await
    }

    /// Check if the content is present in the store.
    async fn content_present(
        &self,
        package_id: &PackageId,
        _digest: &AnyHash,
        version: String,
    ) -> Result<bool, ContentStoreError> {
        let reference = self.reference(package_id, version);
        self.client.content_exists(reference).await
    }
}
