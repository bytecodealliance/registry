use chrono::{DateTime, Utc};
use base64::{Engine as _, engine::{self, general_purpose}};
use std::str::FromStr; // 0.4.15

struct Component;
use bindings::exports::component::validation as validationbindings;
pub use semver::{Version, VersionReq};

use anyhow::Error;
use anyhow::anyhow;

use warg_protocol::{
  package,
  proto_envelope::{ProtoEnvelope, ProtoEnvelopeBody}, 
  SerdeEnvelope,
  registry::{MapCheckpoint, RecordId, LogId, LogLeaf, MapLeaf, PackageId},
};
use warg_crypto::{signing, Decode, hash::{Hash, Sha256, HashAlgorithm, AnyHash}};
use warg_transparency::{log::LogProofBundle, map::MapProofBundle};
use warg_api::v1::proof::ProofError;

fn perm_binding(permission: &package::model::Permission) -> validationbindings::validating::Permission {
  match permission {
      &package::Permission::Release => validationbindings::validating::Permission::Release,
      &package::Permission::Yank => validationbindings::validating::Permission::Yank,
      &_ => validationbindings::validating::Permission::Release,
  }
}

#[derive(Debug, Clone)]
// #[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    /// The name of the package.
    pub name: String,
    /// The last known checkpoint of the package.
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<String>,
    // pub checkpoint: Option<String>,
    /// The current validation state of the package.
    // #[serde(default)]
    pub state: package::LogState,
}

impl PackageInfo {
  /// Creates a new package info for the given package name and url.
  pub fn new(name: impl Into<String>, 
  ) -> Self {
      Self {
          name: name.into(),
          checkpoint: None,
          state: package::LogState::default(),
      }
  }
}

#[derive(Debug)]
struct ProtoBody(validationbindings::validating::ProtoEnvelopeBody);

impl<Content> TryFrom<ProtoBody> for ProtoEnvelope<Content>
where
    Content: Decode,
{
    type Error = Error;

    fn try_from(value: ProtoBody) -> Result<Self, Self::Error> {
        let contents = Content::decode(&value.0.content_bytes)?;
        let envelope = ProtoEnvelope {
            contents,
            content_bytes: value.0.content_bytes,
            key_id: value.0.key_id.into(),
            signature: signing::Signature::from_str(&value.0.signature).unwrap(),
        };
        Ok(envelope)
    }
}

impl bindings::exports::component::validation::validating::Validating for Component {
    fn validate(
        package_records: Vec<validationbindings::validating::ProtoEnvelopeBody>,
    ) -> Result<validationbindings::validating::PackageInfo, ()> {
        let mut package = PackageInfo::new("funny");
        let mut permissions = Vec::new();
        let mut releases = Vec::new();
        let mut keys = Vec::new();
        let mut heads = Vec::with_capacity(1);
        for package_record in package_records {
            let rec: ProtoBody = ProtoBody(package_record);
            let record: Result<ProtoEnvelope<package::PackageRecord>, Error> = rec.try_into();
            let record = record.unwrap();
            let res = package.state.validate(&record);
            for (key, value) in &package.state.permissions {
                permissions.push(validationbindings::validating::PermissionEntry {
                    key_id: key.to_string(),
                    permissions: value
                        .into_iter()
                        .map(|p: &package::model::Permission| perm_binding(p))
                        .collect(),
                })
            }
            for (key, value) in &package.state.releases {
                let t: DateTime<Utc> = value.timestamp.into();
                releases.push(validationbindings::validating::Release {
                    version: key.to_string(),
                    by: value.by.to_string(),
                    timestamp: t.to_rfc3339(),
                    state: match &value.state {
                        package::ReleaseState::Released { content } => {
                            validationbindings::validating::ReleaseState::Released(validationbindings::validating::Released {
                                content: validationbindings::validating::AnyHash {
                                    algo: validationbindings::validating::HashAlgorithm::Sha256,
                                    bytes: content.bytes().to_vec(),
                                },
                            })
                        }
                        package::ReleaseState::Yanked { by, timestamp } => {
                            let ts: DateTime<Utc> = (*timestamp).into();
                            validationbindings::validating::ReleaseState::Yanked(validationbindings::validating::Yanked {
                                by: by.to_string(),
                                timestamp: ts.to_string(),
                            })
                        }
                    },
                })
            }
            for (key, value) in &package.state.keys {
                keys.push(validationbindings::validating::KeyEntry {
                    key_id: key.to_string(),
                    public_key: value.to_string(),
                })
            }
        }
        if let Some(head) = package.state.head() {
            heads.push(validationbindings::validating::LogLeaf {
                log_id: LogId::package_log::<Sha256>(&PackageId::new("foo:bar").unwrap()).to_string(),
                record_id: head.digest.clone().to_string(),
            });
        }
        return Ok(validationbindings::validating::PackageInfo {
            name: package.name,
            checkpoint: package.checkpoint,
            state: validationbindings::validating::Validator {
                algorithm: Some(validationbindings::validating::HashAlgorithm::Sha256),
                head: Some(validationbindings::validating::Head {
                    digest: validationbindings::validating::RecordId::AnyHash(validationbindings::validating::AnyHash {
                        algo: validationbindings::validating::HashAlgorithm::Sha256,
                        bytes: package
                            .state
                            .head
                            .as_ref()
                            .map(|h| h.digest.0.bytes().to_vec())
                            .unwrap(),
                    }),
                    timestamp: package.state.head.map(|h| {
                        let t: DateTime<Utc> = h.timestamp.into();
                        t.to_rfc3339()
                    }),
                }),
                permissions,
                releases,
                keys: Some(keys),
            },
            heads,
        });
    }
}
bindings::export!(Component);
