use chrono::{DateTime, Utc};
use std::str::FromStr; // 0.4.15

use bindings::protocol;
struct Component;
// pub mod operator;
// pub mod package;
// mod proto_envelope;
// pub mod registry;
// mod serde_envelope;

pub use semver::{Version, VersionReq};

use anyhow::Error;
use anyhow::anyhow;

// pub use proto_envelope::{ProtoEnvelope, ProtoEnvelopeBody};
// pub use serde_envelope::SerdeEnvelope;
use warg_protocol::{
  package,
  proto_envelope::{ProtoEnvelope, ProtoEnvelopeBody}, 
  SerdeEnvelope,
  registry::{MapCheckpoint, RecordId, LogId, LogLeaf, MapLeaf},
};
use warg_crypto::{signing, Decode, hash::{Sha256, HashAlgorithm, DynHash}};
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

mod timestamp {
    use serde::Deserializer;
    use serde::{Deserialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(timestamp: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;

        let duration_since_epoch = match timestamp.duration_since(UNIX_EPOCH) {
            Ok(duration_since_epoch) => duration_since_epoch,
            Err(_) => return Err(S::Error::custom("timestamp must be later than UNIX_EPOCH")),
        };

        serializer.serialize_str(&format!(
            "{secs}.{nsecs}",
            secs = duration_since_epoch.as_secs(),
            nsecs = duration_since_epoch.subsec_nanos()
        ))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let s = String::deserialize(deserializer)?;
        let (secs, nsecs) = s
            .split_once('.')
            .ok_or_else(|| D::Error::custom("timestamp must be in the format <secs>.<nsecs>"))?;

        Ok(SystemTime::UNIX_EPOCH
            + Duration::new(
                secs.parse::<u64>().map_err(D::Error::custom)?,
                nsecs.parse::<u32>().map_err(D::Error::custom)?,
            ))
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
      println!("inclusion input {:?}", &input);
      println!("inclusion checkpoint {:?}", &checkpoint);
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
      let log_proof_bundle: LogProofBundle<Sha256, LogLeaf> =
            LogProofBundle::decode(input.log.as_bytes()).unwrap();
        let (log_data, _, log_inclusions) = log_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(log_inclusions.iter()) {
            let found = proof.evaluate_value(
              &log_data,
              &LogLeaf { log_id: LogId(DynHash {
                algo: HashAlgorithm::Sha256,
                bytes: leaf.log_id.as_bytes().to_vec()
              }), record_id: RecordId(DynHash {
                algo: HashAlgorithm::Sha256,
                bytes: leaf.record_id.as_bytes().to_vec()
              }) }).unwrap();
            
            // let root = map_checkpoint.log_root.clone().try_into().unwrap();
            // if found != root {
            //     println!("ERR: {:?}", Err::<ProofError, anyhow::Error>(anyhow!(ProofError::IncorrectProof { root, found })));
            // }
        }
        let map_proof_bundle: MapProofBundle<Sha256, MapLeaf> =
        MapProofBundle::decode(input.map.as_bytes()).unwrap();
        let map_inclusions = map_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(map_inclusions.iter()) {
            let found = proof.evaluate(
                &leaf.log_id.as_bytes(),
                &MapLeaf {
                    record_id: RecordId(DynHash {
                      algo: HashAlgorithm::Sha256,
                      bytes: leaf.record_id.as_bytes().to_vec()
                    })
                },
            );
            // let root = map_checkpoint.map_root.clone().try_into().unwrap();
            // if found != root {
            //     println!("ERR {:?}", Err::ProofError, anyhow::Error>(anyhow!(ProofError::IncorrectProof { root, found })));
            // }
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
          println!("BEFORE TRY {:?}", rec);
          let record: Result<ProtoEnvelope<package::model::PackageRecord>, Error> = rec.try_into();
          println!("the record {:?}", record);
          let record = record.unwrap();
          println!("AFTER TRY");
          let res = package.state.validate(&record);
          println!("THE VALIDATION: {:?}", res);
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
        // else {
        //     return Err("COULDNt DO IT FOR SOME REASON");
        // }
        
        println!("MAYBE WHAT I NEED {:?}", package.state.head);
        println!("MAYBE MORE OF WHAT I NEED {:?}", heads);
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
