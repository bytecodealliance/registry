use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use oci_distribution::config::{Architecture, Config as DistConfig, ConfigFile, Os};
use oci_distribution::{
    client,
    client::{ClientProtocol, Config, ImageLayer},
    manifest::OciImageManifest,
    secrets::RegistryAuth,
    Reference,
};
use serde_json;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Handle;
use tokio::sync::RwLock;
use tokio::task::block_in_place;

use warg_crypto::hash::AnyHash;

use crate::{
    contentstore::ContentStoreError, contentstore::ContentStoreError::ContentStoreInternalError,
};

const COMPONENT_ARTIFACT_TYPE: &str = "application/vnd.bytecodealliance.component.v1+wasm";
const WASM_LAYER_MEDIA_TYPE: &str = "application/vnd.bytecodealliance.wasm.component.layer.v0+wasm";
// const COMPONENT_COMPOSE_MANIFEST_MEDIA_TYPE: &str = "application/vnd.bytecodealliance.component.compose.v0+yaml";

/// Client for interacting with an OCI registry
pub struct Client {
    oci_client: Arc<RwLock<oci_distribution::Client>>,
    auth: RegistryAuth,
    temp_dir: PathBuf,
}

impl Client {
    /// Create a new instance of an OCI client for storing components.
    pub async fn new(insecure: bool, auth: RegistryAuth, temp_dir: &PathBuf) -> Self {
        let client = oci_distribution::Client::new(Self::build_config(insecure));
        Self {
            oci_client: Arc::new(RwLock::new(client)),
            auth: auth.into(),
            temp_dir: temp_dir.clone(),
        }
    }

    pub async fn pull(
        &self,
        reference: impl AsRef<str>,
        digest: &AnyHash,
    ) -> Result<File, ContentStoreError> {
        let path = self.cached_content_path(digest);
        if Path::new(&path)
            .try_exists()
            .map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))?
        {
            let file = File::open(path)
                .await
                .map_err(|e| ContentStoreError::ContentStoreInternalError(e.to_string()))?;
            return Ok(file);
        }

        let reference: Reference = reference
            .as_ref()
            .parse()
            .with_context(|| format!("cannot parse reference {}", reference.as_ref()))
            .map_err(|e| ContentStoreInternalError(e.to_string()))?;

        // TODO: fix the higher-level lifetime error that occurs when not using block_in_place and
        // block_on.
        let result = block_in_place(|| {
            Handle::current().block_on(async move {
                let mut oci = self.oci_client.write().await;
                oci.pull(&reference, &self.auth, vec![WASM_LAYER_MEDIA_TYPE])
                    .await
            })
        });

        let image = result.map_err(|e| ContentStoreInternalError(e.to_string()))?;

        let layer = image
            .layers
            .into_iter()
            .find(|l| l.sha256_digest() == digest.to_string())
            .ok_or(ContentStoreInternalError("layer not found".to_string()))?;
        let mut file = File::create(self.cached_content_path(digest))
            .await
            .map_err(|e| ContentStoreInternalError(e.to_string()))?;
        file.write_all(&layer.data)
            .await
            .map_err(|e| ContentStoreInternalError(e.to_string()))?;
        Ok(file)
    }

    /// Push a component to an OCI registry.
    pub async fn push(
        &self,
        reference: impl AsRef<str>,
        file: &mut File,
        digest: &AnyHash,
    ) -> Result<String, ContentStoreError> {
        let reference: Reference = reference
            .as_ref()
            .parse()
            .with_context(|| format!("cannot parse reference {}", reference.as_ref()))
            .map_err(|e| ContentStoreInternalError(e.to_string()))?;

        let entrypoint = format!("/{}", digest.to_string().strip_prefix("sha256:").unwrap());
        let config = ConfigFile {
            architecture: Architecture::Wasm,
            os: Os::Wasi,
            config: Some(DistConfig {
                // use the sha256 hash as the file name for the entrypoint
                entrypoint: vec![entrypoint],
                ..Default::default()
            }),
            ..Default::default()
        };
        let config_data =
            serde_json::to_vec(&config).map_err(|e| ContentStoreInternalError(e.to_string()))?;
        let oci_config = Config::oci_v1(config_data, None);
        let mut layers = Vec::new();
        let wasm_layer = Self::wasm_layer(file)
            .await
            .context("cannot create wasm layer")
            .map_err(|e| ContentStoreInternalError(e.to_string()))?;
        layers.insert(0, wasm_layer);
        let mut manifest = OciImageManifest::build(&layers, &oci_config, None);
        manifest.artifact_type = Some(COMPONENT_ARTIFACT_TYPE.to_string());

        // TODO: fix the higher-level lifetime error that occurs when not using block_in_place and
        // block_on.
        let result = block_in_place(|| {
            Handle::current().block_on(async move {
                tracing::log::trace!("Pushing component to {:?}", reference);
                let mut oci = self.oci_client.write().await;
                oci.push(&reference, &layers, oci_config, &self.auth, Some(manifest))
                    .await
            })
        });

        result
            .map(|push_response| push_response.manifest_url)
            .context("cannot push component to the registry")
            .map_err(|e| ContentStoreInternalError(e.to_string()))
    }

    pub async fn content_exists(
        &self,
        reference: impl AsRef<str>,
    ) -> Result<bool, ContentStoreError> {
        let reference: Reference = reference
            .as_ref()
            .parse()
            .with_context(|| format!("cannot parse reference {}", reference.as_ref()))
            .map_err(|e| ContentStoreInternalError(e.to_string()))
            .unwrap();

        let mut oci = self.oci_client.write().await;
        match oci.fetch_manifest_digest(&reference, &self.auth).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Create a new wasm layer based on a file.
    async fn wasm_layer(file: &mut File) -> Result<ImageLayer> {
        tracing::log::trace!("Reading wasm component from {:?}", file);

        let mut contents = vec![];
        file.read_to_end(&mut contents)
            .await
            .context("cannot read wasm component")?;

        Ok(ImageLayer::new(
            contents,
            WASM_LAYER_MEDIA_TYPE.to_string(),
            None,
        ))
    }

    /// Returns the path to the content file for a given content address.
    fn cached_content_path(&self, digest: &AnyHash) -> PathBuf {
        self.temp_dir.join(Self::content_file_name(digest))
    }

    /// Returns the file name for a given content address replacing colons with dashes.
    fn content_file_name(digest: &AnyHash) -> String {
        digest.to_string().replace(':', "-")
    }

    /// Build the OCI client configuration given the insecure option.
    fn build_config(insecure: bool) -> client::ClientConfig {
        let protocol = if insecure {
            ClientProtocol::Http
        } else {
            ClientProtocol::Https
        };

        client::ClientConfig {
            protocol,
            ..Default::default()
        }
    }
}
