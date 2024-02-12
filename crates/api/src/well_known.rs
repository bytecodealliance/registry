//! Types relating to the .well-known endpoint.
use serde::{Deserialize, Serialize, Serializer};
use std::{borrow::Cow, collections::HashMap};

use thiserror::Error;

#[derive(Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
enum RawError<'a> {
    Message { status: u16, message: Cow<'a, str> },
}

/// Represents a .well_known error
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum WellKnownError {
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The HTTP status code.
        status: u16,
        /// The error message
        message: String,
    },
}

impl WellKnownError {
    /// Returns the HTTP status code of the error.
    pub fn status(&self) -> u16 {
        match self {
            // Self::CheckpointNotFound(_) | Self::LogNotFound(_) | Self::FetchTokenNotFound(_) => 404,
            Self::Message { status, .. } => *status,
        }
    }
}

impl Serialize for WellKnownError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            WellKnownError::Message { status, message } => RawError::Message {
                status: *status,
                message: message.into(),
            }
            .serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for WellKnownError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawError::deserialize(deserializer)? {
            RawError::Message { status, message } => Ok(Self::Message {
                status,
                message: message.to_string(),
            }),
        }
    }
}

/// Represents the `.well-known/warg/registry.json` configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WellKnown {
    /// OCI registry configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oci: Option<OciRegistry>,
    /// Warg registry configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warg: Option<WargRegistry>,
    /// Other namespaces for registry mapping
    pub namespaces: HashMap<String, Registry>,
}

/// Registry configuration for OCI and Warg
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Registry {
    /// OCI mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oci: Option<OciRegistry>,
    /// Warg mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warg: Option<WargRegistry>,
}

/// OCI registry configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OciRegistry {
    /// Domain option
    #[serde(flatten)]
    pub domain_option: DomainOption,
    /// Optional namespace prefix for OCI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace_prefix: Option<String>,
}

/// Warg registry configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WargRegistry {
    /// Domain option
    #[serde(flatten)]
    pub domain_option: DomainOption,
}

/// Domain or namespace as subdomain
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DomainOption {
    /// Domain
    Domain(String),
    /// Namespace used as the subdomain of this domain
    NamespaceAsSubdomainOf(String),
}
