use chrono::{DateTime, Utc};
use base64::{Engine as _, engine::{self, general_purpose}};
use std::str::FromStr; // 0.4.15

use bindings::protocol;
struct Component;

pub use semver::{Version, VersionReq};

use anyhow::Error;
use anyhow::anyhow;

use warg_protocol::{
  package,
  proto_envelope::{ProtoEnvelope, ProtoEnvelopeBody}, 
  SerdeEnvelope,
  registry::{MapCheckpoint, RecordId, LogId, LogLeaf, MapLeaf},
};
use warg_crypto::{signing, Decode, hash::{Hash, Sha256, HashAlgorithm, DynHash}};
use warg_transparency::{log::LogProofBundle, map::MapProofBundle};
use warg_api::proof::ProofError;

/// Represents information about a registry package.
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
    pub state: package::Validator,
}

impl PackageInfo {
    /// Creates a new package info for the given package name and url.
    pub fn new(name: impl Into<String>, // checkpoint: protocol::MapCheckpoint
    ) -> Self {
        Self {
            name: name.into(),
            checkpoint: None,
            state: package::Validator::default(),
        }
    }
}

#[derive(Debug)]
struct MyBody(protocol::ProtoEnvelopeBody);

impl<Content> TryFrom<MyBody> for ProtoEnvelope<Content>
where
    Content: Decode,
{
    type Error = Error;

    fn try_from(value: MyBody) -> Result<Self, Self::Error> {
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

fn perm_binding(permission: &package::model::Permission) -> protocol::Permission {
    match permission {
        &package::Permission::Release => protocol::Permission::Release,
        &package::Permission::Yank => protocol::Permission::Yank,
        &_ => protocol::Permission::Release,
    }
}

impl protocol::Protocol for Component {
    fn prove_inclusion(input: protocol::Inclusion, checkpoint: protocol::MapCheckpoint, heads: Vec<protocol::LogLeaf>) {
      let map_checkpoint = MapCheckpoint {
        log_root: DynHash {
          algo: HashAlgorithm::Sha256,
          bytes: checkpoint.log_root.as_bytes().to_vec(),
        },
        log_length: checkpoint.log_length,
        map_root: DynHash {
          algo: HashAlgorithm::Sha256,
          bytes: checkpoint.map_root.as_bytes().to_vec()
        }
      };
      let bytes = general_purpose::STANDARD.decode(&input.log).unwrap();
      let log_proof_bundle: LogProofBundle<Sha256, LogLeaf> =
            LogProofBundle::decode(general_purpose::STANDARD
              .decode(&input.log).unwrap().as_slice()).unwrap();
        let (log_data, _, log_inclusions) = log_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(log_inclusions.iter()) {
          let leaf = &LogLeaf {
            log_id: LogId(DynHash::from_str(&leaf.log_id).unwrap()),
            record_id: RecordId(DynHash::from_str(&leaf.record_id).unwrap())
          };
            let found = proof.evaluate_value(
              &log_data,
              &leaf).unwrap();
            let root: Hash<Sha256> = DynHash::from_str(&checkpoint.log_root).expect("SOMETHING").clone().try_into().unwrap();
            if found != root {
                println!("ERR: {:?}", Err::<ProofError, anyhow::Error>(anyhow!(ProofError::IncorrectProof { root, found })));
            }
        }
        let map_proof_bundle: MapProofBundle<Sha256, MapLeaf> =
        MapProofBundle::decode(general_purpose::STANDARD
          .decode(&input.map).unwrap().as_slice()).unwrap();
        let map_inclusions = map_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(map_inclusions.iter()) {
            let map_found = proof.evaluate(
                &LogId(DynHash::from_str(&leaf.log_id).unwrap()),
                &MapLeaf {
                    record_id: RecordId(DynHash::from_str(&leaf.record_id).unwrap()) 
                },
              );
            let map_root: Hash<Sha256> = DynHash::from_str(&checkpoint.map_root).expect("SOMETHING").clone().try_into().unwrap();
            if map_found != map_root {
                println!("ERR {:?}", Err::<ProofError, anyhow::Error>(anyhow!(ProofError::IncorrectProof { root: map_root, found: map_found })));
            }
        }
    }
    fn validate(
        package_records: Vec<protocol::ProtoEnvelopeBody>,
    ) -> protocol::PackageInfo {
        let mut package = PackageInfo::new("funny");
        let mut permissions = Vec::new();
        let mut releases = Vec::new();
        let mut keys = Vec::new();
        let mut heads = Vec::with_capacity(1);
        for package_record in package_records {
          let rec: MyBody = MyBody(package_record);
          let record: Result<ProtoEnvelope<package::model::PackageRecord>, Error> = rec.try_into();
          let record = record.unwrap();
          let res = package.state.validate(&record);
          for (key, value) in &package.state.permissions {
              permissions.push(protocol::PermissionEntry {
                  key_id: key.to_string(),
                  permissions: value
                      .into_iter()
                      .map(|p: &package::model::Permission| perm_binding(p))
                      .collect(),
              })
          }
          for (key, value) in &package.state.releases {
            let t: DateTime<Utc> = value.timestamp.into();
            releases.push(protocol::Release {
              version: key.to_string(),
              by: value.by.to_string(),
              timestamp: t.to_rfc3339(),
              state: match &value.state {
                package::ReleaseState::Released{ content } => protocol::ReleaseState::Released(protocol::Released {
                  content: protocol::DynHash {
                    algo: protocol::HashAlgorithm::Sha256,
                    bytes: content.bytes().to_vec()
                  }
                }),
                package::ReleaseState::Yanked{ by, timestamp } => {
                  let ts: DateTime<Utc> = (*timestamp).into();
                  protocol::ReleaseState::Yanked(protocol::Yanked {
                    by: by.to_string(),
                    timestamp: ts.to_string()
                  })
                }
              }
            })
          }
          for (key, value) in &package.state.keys {
              keys.push(protocol::KeyEntry {
                  key_id: key.to_string(),
                  public_key: value.to_string(),
              })
          }
        }
        if let Some(head) = package.state.head() {
          heads.push(protocol::LogLeaf {
              log_id: LogId::package_log::<Sha256>("funny").to_string(),
              record_id: head.digest.clone().to_string(),
          });
        } 
        return protocol::PackageInfo {
            name: package.name,
            checkpoint: package.checkpoint,
            state: protocol::Validator {
                algorithm: Some(protocol::HashAlgorithm::Sha256),
                head: Some(protocol::Head {
                    digest: protocol::RecordId::DynHash(protocol::DynHash {
                        algo: protocol::HashAlgorithm::Sha256,
                        bytes: package
                            .state
                            .head
                            .as_ref()
                            .map(|h| h.digest.0.bytes().to_vec()).unwrap(),
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
            heads
        };
    }
}

bindings::export!(Component);
