use crate::contentstore::ContentStoreError::ContentStoreInternalError;
use crate::contentstore::{
    ContentStore, ContentStoreError, ContentStoreUriSigning, PresignedContentStore,
};
use aws_credential_types::Credentials;
use aws_sdk_s3;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use axum::http::HeaderValue;
use futures::TryStreamExt;
use hyper::client::HttpConnector;
use hyper::Uri;
use hyper_proxy::{Intercept, Proxy, ProxyConnector};
use secrecy::{ExposeSecret, SecretString};
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;
use url::Url;
use warg_crypto::hash::AnyHash;
use warg_protocol::registry::PackageId;

#[derive(Debug, Clone)]
pub struct S3ContentStore {
    client: aws_sdk_s3::Client,
    // on R2 there is a max number of buckets per account (default: 1000)
    bucket_name: String,
    presign_ttl: u64,
    temp_dir: PathBuf,
}

impl S3ContentStore {
    pub async fn new(
        endpoint: Url,
        access_key_id: SecretString,
        access_key_secret: SecretString,
        region: String,
        bucket_name: String,
        temp_dir: &PathBuf,
        presign_ttl: u64,
    ) -> Self {
        let creds = Credentials::new(
            access_key_id.expose_secret().to_string(),
            access_key_secret.expose_secret().to_string(),
            None,
            None,
            "warg-s3-static-provider",
        );
        let config_builder = aws_config::from_env()
            .region(Region::new(region.clone()))
            .endpoint_url(endpoint.clone())
            .credentials_provider(creds);
        let config = match env::var("HTTP_PROXY") {
            Ok(proxy_env) => {
                let proxy_str = proxy_env.as_str();
                let proxy_uri = Uri::from_str(proxy_str).unwrap();
                let proxy_connector = {
                    let proxy = Proxy::new(Intercept::All, proxy_uri.clone());
                    let connector = HttpConnector::new();
                    ProxyConnector::from_proxy(connector, proxy).unwrap()
                };
                // need to ensure the `hyper` feature of smithy-client is enabled
                let hyper_client =
                    aws_smithy_client::hyper_ext::Adapter::builder().build(proxy_connector);
                info!("Using proxy {}", proxy_str);
                aws_sdk_s3::config::Builder::from(&config_builder.load().await)
                    .http_connector(hyper_client)
                    .build()
            }
            _ => (&config_builder.load().await).into(),
        };
        let client = aws_sdk_s3::Client::from_conf(config);
        Self {
            client,
            bucket_name,
            temp_dir: temp_dir.clone(),
            presign_ttl,
        }
    }

    /// Returns the path to the content file for a given content address.
    fn cached_content_path(&self, digest: &AnyHash) -> PathBuf {
        self.temp_dir.join(Self::content_file_name(digest))
    }

    /// Returns the file name for a given content address replacing colons with dashes.
    fn content_file_name(digest: &AnyHash) -> String {
        digest.to_string().replace(':', "-")
    }

    fn content_store_name(package_id: &PackageId, version: &str, digest: &AnyHash) -> String {
        format!(
            "{}-{}-{}-{}",
            package_id.namespace(),
            package_id.name(),
            version,
            Self::content_file_name(digest)
        )
    }

    fn get_presign_config(&self) -> Result<PresigningConfig, ContentStoreError> {
        Ok(PresigningConfig::builder()
            .expires_in(Duration::from_secs(self.presign_ttl))
            .build()
            .map_err(|e| {
                ContentStoreInternalError(format!("cannot build presigning config: {}", e))
            })?)
    }
}

#[axum::async_trait]
impl ContentStore for S3ContentStore {
    async fn fetch_content(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
    ) -> Result<File, ContentStoreError> {
        let path = self.cached_content_path(digest);
        if Path::new(&path)
            .try_exists()
            .map_err(|e| ContentStoreInternalError(e.to_string()))?
        {
            let file = File::open(path)
                .await
                .map_err(|e| ContentStoreInternalError(e.to_string()))?;
            return Ok(file);
        }

        let mut object = self
            .client
            .get_object()
            .bucket(self.bucket_name.clone())
            .key(S3ContentStore::content_store_name(
                package_id, &version, digest,
            ))
            .send()
            .await
            .map_err(|e| ContentStoreInternalError(e.to_string()))?;

        let mut file = File::create(path)
            .await
            .map_err(|e| ContentStoreInternalError(e.to_string()))?;

        while let Some(bits) = object
            .body
            .try_next()
            .await
            .map_err(|e| ContentStoreInternalError(e.to_string()))?
        {
            file.write_all(&bits)
                .await
                .map_err(|e| ContentStoreInternalError(e.to_string()))?;
        }
        Ok(file)
    }

    async fn store_content(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
        content: &mut File,
    ) -> Result<String, ContentStoreError> {
        let mut buf: Vec<u8> = Vec::new();
        content.read_to_end(&mut buf).await.map_err(|e| {
            ContentStoreInternalError(format!(
                "cannot read content for package {} version {}: {}",
                package_id,
                version.clone(),
                e
            ))
        })?;

        self.client
            .put_object()
            .bucket(self.bucket_name.clone())
            .body(ByteStream::from(buf))
            .key(S3ContentStore::content_store_name(
                package_id, &version, digest,
            ))
            .content_type("application/wasm".to_string())
            .customize()
            .await
            .map_err(|e| {
                ContentStoreInternalError(format!(
                    "cannot customize request for package {} version {}: {}",
                    package_id,
                    version.clone(),
                    e
                ))
            })?
            // add a header so that Cloudflare will automatically create the bucket if it doesn't exist
            .mutate_request(|req| {
                req.headers_mut().insert(
                    "cf-create-bucket-if-missing",
                    HeaderValue::from_static("true"),
                );
            })
            .send()
            .await
            .map_err(|e| {
                ContentStoreInternalError(format!(
                    "cannot store content for package {} version {}: {}",
                    package_id, version, e
                ))
            })?;

        Ok(digest.to_string())
    }

    async fn content_present(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
    ) -> Result<bool, ContentStoreError> {
        self.client
            .head_object()
            .bucket(self.bucket_name.clone())
            .key(S3ContentStore::content_store_name(
                package_id, &version, digest,
            ))
            .send()
            .await
            .map(|_| true)
            .or_else(|_| Ok(false))
    }

    async fn uri_signing(&self) -> ContentStoreUriSigning {
        ContentStoreUriSigning::Presigned(Box::new(self.clone()))
    }
}

#[axum::async_trait]
impl PresignedContentStore for S3ContentStore {
    async fn read_uri(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
    ) -> Result<Uri, ContentStoreError> {
        let config = self.get_presign_config()?;

        let presigned = self
            .client
            .get_object()
            .bucket(self.bucket_name.clone())
            .key(S3ContentStore::content_store_name(
                package_id, &version, digest,
            ))
            .presigned(config)
            .await
            .map_err(|e| {
                ContentStoreInternalError(format!("cannot generate presigned Uri: {e}"))
            })?;

        Ok(presigned.uri().clone())
    }

    async fn write_uri(
        &self,
        package_id: &PackageId,
        digest: &AnyHash,
        version: String,
    ) -> Result<Uri, ContentStoreError> {
        let config = self.get_presign_config()?;

        let presigned = self
            .client
            .put_object()
            .bucket(self.bucket_name.clone())
            .key(S3ContentStore::content_store_name(
                package_id, &version, digest,
            ))
            .presigned(config)
            .await
            .map_err(|e| {
                ContentStoreInternalError(format!("cannot generate presigned Uri: {e}"))
            })?;

        Ok(presigned.uri().clone())
    }
}
