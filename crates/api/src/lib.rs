//! The serializable types for the Warg REST API.
#![deny(missing_docs)]

pub mod v1;

use serde::{de::Unexpected, Deserialize, Serialize};

/// Relative URL path for the `WellKnownConfig`.
pub const WELL_KNOWN_PATH: &str = ".well-known/wasm-pkg/registry.json";

/// This config allows a domain to point to another URL where the registry
/// API is hosted.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellKnownConfig {
    /// For OCI registries, the domain name where the registry is hosted.
    pub oci_registry: Option<String>,
    /// For OCI registries, a name prefix to use before the namespace.
    pub oci_namespace_prefix: Option<String>,
    /// For Warg registries, the URL where the registry is hosted.
    pub warg_url: Option<String>,
}

/// A utility type for serializing and deserializing constant status codes.
struct Status<const CODE: u16>;

impl<const CODE: u16> Serialize for Status<CODE> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u16(CODE)
    }
}

impl<'de, const CODE: u16> Deserialize<'de> for Status<CODE> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = u16::deserialize(deserializer)?;
        if code == CODE {
            Ok(Status::<CODE>)
        } else {
            Err(serde::de::Error::invalid_value(
                Unexpected::Unsigned(code as u64),
                &"a matching status code",
            ))
        }
    }
}
