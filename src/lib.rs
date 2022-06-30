use std::borrow::Cow;

#[cfg(feature = "client")]
pub mod client;
pub mod digest;
pub mod dsse;
pub mod release;
#[cfg(feature = "server")]
pub mod server;

mod maintainer;
mod serde;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    AsyncIoError(#[from] futures::io::Error),

    #[error("invalid contentDigest: {0}")]
    InvalidContentDigest(Cow<'static, str>),

    #[error("invalid content source: {0}")]
    InvalidContentSource(Cow<'static, str>),

    #[error("invalid name: {0}")]
    InvalidEntityName(Cow<'static, str>),

    #[error("invalid entityType: {0}")]
    InvalidEntityType(Cow<'static, str>),

    #[error("invalid signature: {0}")]
    InvalidSignature(Cow<'static, str>),

    #[error("invalid signature key: {0}")]
    InvalidSignatureKey(Cow<'static, str>),

    #[error("invalid version: {0}")]
    InvalidVersion(#[from] semver::Error),

    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("release already exists")]
    ReleaseAlreadyExists,

    #[error("signature error: {0}")]
    SignatureError(#[from] signature::Error),
}
