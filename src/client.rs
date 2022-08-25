use std::{borrow::Cow, collections::HashMap};

use bytes::Bytes;
use http_client::{
    http_types::url::{ParseError as UrlParseError, Url},
    Body, HttpClient, Request, Response,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    dsse::Signature,
    maintainer::{MaintainerKey, MaintainerPublicKey, MaintainerSecret},
    release::{
        EntityName, EntityType, PublishRelease, Release, ReleaseManifest, UnpublishedRelease,
    },
    Error,
};

#[derive(Debug)]
pub struct Client<C: HttpClient> {
    base: Url,
    http: C,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("http: {0}")]
    HttpError(http_client::Error),

    #[error("{0}")]
    RegistryError(#[from] crate::Error),

    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] UrlParseError),

    #[error("{0}")]
    Other(Cow<'static, str>),
}

impl From<http_client::Error> for ClientError {
    fn from(err: http_client::Error) -> Self {
        Self::HttpError(err)
    }
}

impl<C: HttpClient> Client<C> {
    pub fn new(http: C, base_url: impl Into<Url>) -> Self {
        Self {
            http,
            base: base_url.into(),
        }
    }

    pub async fn create_unpublished_release(
        &self,
        release: &ReleaseManifest,
    ) -> Result<UnpublishedRelease, ClientError> {
        let release_json = serde_json::to_string(release).map_err(Error::from)?;
        let url = self.base.join("releases/")?;
        let mut req = Request::post(url);
        req.set_body(release_json.clone());
        let resp = self.http.send(req).await?;
        let mut resp = response_error(resp).await?;
        let unpublished: UnpublishedRelease = resp.body_json().await?;
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
        let mut req = Request::post(url);
        req.set_body(content);
        let resp = self.http.send(req).await?;
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
        package_type: EntityType,
        name: EntityName,
        version: semver::Version,
    ) -> Result<Release, ClientError> {
        let path = Release::build_resource_path(&package_type, &name, &version);
        self.get_json(&path).await
    }

    // TODO: return a stream? AsyncRead?
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
        let req = Request::get(url);
        let mut resp = response_error(self.http.send(req).await?).await?;
        let content: Bytes = resp.body_bytes().await?.into();

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
        let req = Request::get(url);
        let mut resp = response_error(self.http.send(req).await?).await?;
        Ok(resp.body_json().await?)
    }

    async fn post_json<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<T, ClientError> {
        let url = self.base.join(path)?;
        let mut req = Request::post(url);
        req.set_body(Body::from_json(body)?);
        let mut resp = response_error(self.http.send(req).await?).await?;
        Ok(resp.body_json().await?)
    }

    fn parse_rel_or_abs_url(&self, rel_or_abs: &str) -> Result<Url, UrlParseError> {
        match rel_or_abs.parse() {
            Ok(url) => Ok(url),
            Err(UrlParseError::RelativeUrlWithoutBase) => self.base.join(rel_or_abs),
            Err(err) => Err(err),
        }
    }
}

async fn response_error(mut resp: Response) -> Result<Response, ClientError> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else {
        // TODO: map responses back to more specific errors (RFC 7807?)
        let detail = resp.body_string().await?;
        Err(ClientError::Other(
            format!("[{}] {}", status, detail).into(),
        ))
    }
}
