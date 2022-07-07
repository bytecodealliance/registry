pub mod client;
pub mod digest;
pub mod dsse;
pub mod maintainer;
pub mod release;

mod serde;

use std::borrow::Cow;

// TODO: verify whether we want to bake in these semantics
pub type Version = semver::Version;

type ErrorMsg = Cow<'static, str>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    AsyncIoError(#[from] futures::io::Error),

    #[error("invalid content digest: {0}")]
    InvalidContentDigest(ErrorMsg),

    #[error("invalid content source: {0}")]
    InvalidContentSource(ErrorMsg),

    #[error("invalid name: {0}")]
    InvalidEntityName(ErrorMsg),

    #[error("invalid entityType: {0}")]
    InvalidEntityType(ErrorMsg),

    #[error("invalid signature: {0}")]
    InvalidSignature(ErrorMsg),

    #[error("invalid signature key: {0}")]
    InvalidSignatureKey(ErrorMsg),

    #[error("invalid version: {0}")]
    InvalidVersion(#[from] semver::Error),

    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("release already exists")]
    ReleaseAlreadyExists,

    #[error("signature error: {0}")]
    SignatureError(#[from] signature::Error),
}
