use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{digest::TypedDigest, dsse::Signature, maintainer::MaintainerPublicKey, Error};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    Component,
}

impl EntityType {
    fn collection_name(&self) -> &'static str {
        match self {
            EntityType::Component => "components",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct EntityName(String);

impl AsRef<str> for EntityName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<EntityName> for String {
    fn from(name: EntityName) -> Self {
        name.0
    }
}

impl TryFrom<String> for EntityName {
    type Error = Error;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        // TODO: more validations
        if name.is_empty() {
            return Err(Error::InvalidEntityName("empty".into()));
        }
        if name.contains('/') {
            return Err(Error::InvalidEntityName("may not contain '/'".into()));
        }
        Ok(Self(name))
    }
}

impl FromStr for EntityName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.to_string().try_into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseManifest {
    pub entity_type: EntityType,
    pub name: EntityName,
    // TODO: Do we want to enforce semver at this level? Is this implementation acceptable?
    pub version: semver::Version,
    pub content_digest: TypedDigest,
}

impl ReleaseManifest {
    pub fn resource_path(&self) -> String {
        Release::build_resource_path(&self.entity_type, &self.name, &self.version)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnpublishedReleaseStatus {
    Pending,
    Processing,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpublishedRelease {
    pub release: String,
    pub status: UnpublishedReleaseStatus,
    // TODO: use RFC 7807 "problem details"?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_url: Option<String>,
}

pub const RELEASE_PAYLOAD_TYPE: &str = "WASM-COMPONENT-REGISTRY-RELEASE";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishRelease {
    pub signature: Signature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub release: String,
    pub release_signature: Signature,
    pub content_sources: Vec<ContentSource>,
}

impl Release {
    pub fn verify_signature(
        &self,
        public_key: &MaintainerPublicKey,
    ) -> Result<ReleaseManifest, Error> {
        public_key.verify_payload(
            RELEASE_PAYLOAD_TYPE,
            self.release.as_bytes(),
            &self.release_signature,
        )?;
        Ok(serde_json::from_str(&self.release)?)
    }

    pub fn build_resource_path(
        entity_type: &EntityType,
        name: &EntityName,
        version: &semver::Version,
    ) -> String {
        format!(
            "/{}/{}/v{}",
            entity_type.collection_name(),
            name.as_ref(),
            version
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentSource {
    pub url: String,
}
