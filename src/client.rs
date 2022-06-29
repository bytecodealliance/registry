use std::borrow::Cow;

use bytes::Bytes;
use reqwest::{Body, Response, Url};
use url::ParseError;

use crate::{
    dsse::Signature,
    maintainer::{MaintainerKey, MaintainerSecretKey},
    release::{PublishRelease, Release, ReleaseManifest, UnpublishedRelease},
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

    #[error("json: {0}")]
    JsonError(#[from] serde_json::Error),

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

    pub async fn register_generated_maintainer_key(
        &self,
    ) -> Result<(MaintainerKey, MaintainerSecretKey), ClientError> {
        let secret_key = MaintainerSecretKey::generate();
        let maintainer_key = MaintainerKey {
            id: "".to_string(),
            public_key: secret_key.public_key(),
        };
        let url = self.base.join("prototype/register-publisher")?;
        let resp = self.http.post(url).json(&maintainer_key).send().await?;
        let resp = response_error(resp).await?;
        Ok((resp.json().await?, secret_key))
    }

    pub async fn create_unpublished_release(
        &self,
        release: &ReleaseManifest,
    ) -> Result<UnpublishedRelease, ClientError> {
        let release_json: Bytes = serde_json::to_vec(release)?.into();
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
        let upload_url = upload_url.as_ref();
        let url: Url = match upload_url.parse() {
            Ok(abs_url) => Ok(abs_url),
            Err(ParseError::RelativeUrlWithoutBase) => self.base.join(upload_url),
            Err(err) => return Err(err.into()),
        }?;
        let resp = self.http.post(url).body(content).send().await?;
        response_error(resp).await?;
        Ok(())
    }

    pub async fn publish(
        &self,
        release: &ReleaseManifest,
        signature: Signature,
    ) -> Result<Release, ClientError> {
        let url = self
            .base
            .join(&format!("{}/publish", release.resource_path()))?;
        let publish = PublishRelease { signature };
        let resp = self.http.post(url).json(&publish).send().await?;
        let resp = response_error(resp).await?;
        Ok(resp.json().await?)
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
