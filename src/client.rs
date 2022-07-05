use std::{borrow::Cow, collections::HashMap};

use bytes::Bytes;
use reqwest::{Body, Response, Url};
use serde::{de::DeserializeOwned, Serialize};
use url::ParseError;

use crate::{
    dsse::Signature,
    maintainer::{MaintainerKey, MaintainerPublicKey, MaintainerSecret},
    release::{
        EntityName, EntityType, PublishRelease, Release, ReleaseManifest, UnpublishedRelease,
    },
    Error,
};

#[derive(Debug)]
pub struct Client {
    base: Url,
    http: reqwest::Client,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("http: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("{0}")]
    RegistryError(#[from] crate::Error),

    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("{0}")]
    Other(Cow<'static, str>),
}

impl Client {
    pub fn new(base_url: impl Into<Url>) -> Self {
        Self {
            base: base_url.into(),
            http: Default::default(),
        }
    }

    pub async fn create_unpublished_release(
        &self,
        release: &ReleaseManifest,
    ) -> Result<UnpublishedRelease, ClientError> {
        let release_json: Bytes = serde_json::to_vec(release).map_err(Error::from)?.into();
        let url = self.base.join("releases/")?;
        let resp = self
            .http
            .post(url)
            .body(release_json.clone())
            .send()
            .await?;
        let resp = response_error(resp).await?;
        let unpublished: UnpublishedRelease = resp.json().await?;
        if unpublished.release != release_json {
            return Err(ClientError::Other(
                "Unpublished release manifest doesn't match request".into(),
            ));
        }
        Ok(unpublished)
    }

    pub async fn upload_content(
        &self,
        upload_url: impl AsRef<str>,
        content: impl Into<Body>,
    ) -> Result<(), ClientError> {
        let url = self.parse_rel_or_abs_url(upload_url.as_ref())?;
        let resp = self.http.post(url).body(content).send().await?;
        response_error(resp).await?;
        Ok(())
    }

    pub async fn publish(
        &self,
        release: &ReleaseManifest,
        signature: Signature,
    ) -> Result<Release, ClientError> {
        let path = format!("{}/publish", release.resource_path());
        let publish = PublishRelease { signature };
        self.post_json(&path, &publish).await
    }

    pub async fn get_release(
        &self,
        entity_type: EntityType,
        name: EntityName,
        version: semver::Version,
    ) -> Result<Release, ClientError> {
        let path = Release::build_resource_path(&entity_type, &name, &version);
        self.get_json(&path).await
    }

    // TODO: return a stream?
    pub async fn fetch_validate_content(&self, release: &Release) -> Result<Bytes, ClientError> {
        // FIXME: simplistic maintainer key lookup
        let key_id = release
            .release_signature
            .key_id
            .as_deref()
            .ok_or_else(|| Error::InvalidSignature("no key ID".into()))?;
        let maintainer_public_keys = self.get_maintainer_public_keys().await?;
        let public_key = maintainer_public_keys.get(key_id).ok_or_else(|| {
            Error::InvalidSignature(format!("no key with ID {:?}", key_id).into())
        })?;

        // Verify release signature
        let release_manifest = release.verify_signature(public_key)?;

        // Fetch content
        let source = release
            .content_sources
            .get(0)
            .ok_or_else(|| Error::InvalidContentSource("no contentSources".into()))?;
        let url = self.parse_rel_or_abs_url(&source.url)?;
        let req = self.http.get(url);
        let resp = response_error(req.send().await?).await?;
        let content = resp.bytes().await?;

        // Verify content digest
        release_manifest
            .content_digest
            .verify_content(&mut content.as_ref())
            .await?;

        Ok(content)
    }

    // PROTOTYPE endpoints

    pub async fn register_generated_maintainer_key(
        &self,
    ) -> Result<(MaintainerKey, MaintainerSecret), ClientError> {
        let mut generated_secret = MaintainerSecret::generate();
        let registration_key = MaintainerKey {
            id: "".to_string(),
            public_key: generated_secret.public_key(),
        };

        let maintainer_key: MaintainerKey = self
            .post_json("prototype/register-maintainer-key", &registration_key)
            .await?;

        if maintainer_key.public_key != registration_key.public_key {
            return Err(Error::InvalidSignatureKey(
                "registered public key doesn't match request".into(),
            )
            .into());
        }
        generated_secret.key_id = maintainer_key.id.clone();

        Ok((maintainer_key, generated_secret))
    }

    async fn get_maintainer_public_keys(
        &self,
    ) -> Result<HashMap<String, MaintainerPublicKey>, ClientError> {
        self.get_json("prototype/maintainer-public-keys").await
    }

    // Simple request helpers

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let url = self.base.join(path)?;
        let req = self.http.get(url);
        let resp = response_error(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    async fn post_json<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<T, ClientError> {
        let url = self.base.join(path)?;
        let req = self.http.post(url).json(body);
        let resp = response_error(req.send().await?).await?;
        Ok(resp.json().await?)
    }

    fn parse_rel_or_abs_url(&self, rel_or_abs: &str) -> Result<Url, ParseError> {
        match rel_or_abs.parse() {
            Ok(url) => Ok(url),
            Err(ParseError::RelativeUrlWithoutBase) => self.base.join(rel_or_abs),
            Err(err) => Err(err),
        }
    }
}

async fn response_error(resp: Response) -> Result<Response, ClientError> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else {
        // TODO: map responses back to more specific errors (RFC 7807?)
        let detail = resp
            .bytes()
            .await
            .map(|body| String::from_utf8_lossy(&body).to_string())
            .unwrap_or_else(|err| format!("failed to read error detail: {:?}", err));
        Err(ClientError::Other(
            format!("[{}] {}", status, detail).into(),
        ))
    }
}
