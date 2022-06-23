use std::borrow::Cow;

#[cfg(feature = "client")]
pub mod client;
pub mod digest;
pub mod dsse;
pub mod release;
#[cfg(feature = "server")]
pub mod server;

mod publisher;
mod serde;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("release already exists")]
    ReleaseAlreadyExists,

    #[error("invalid contentDigest: {0}")]
    InvalidContentDigest(Cow<'static, str>),

    #[error("invalid name: {0}")]
    InvalidEntityName(Cow<'static, str>),

    #[error("invalid entityType: {0}")]
    InvalidEntityType(Cow<'static, str>),

    #[error("invalid signature: {0}")]
    InvalidSignature(Cow<'static, str>),

    #[error("invalid signing key: {0}")]
    InvalidSigningKey(Cow<'static, str>),

    #[error("signing error: {0}")]
    SigningError(#[from] signature::Error),
}
