use std::{fmt, time::SystemTime};

use serde::{Deserialize, Serialize};

use std::{path::PathBuf, sync::Arc};

use anyhow::{Error, Result};
use clap::Subcommand;
use warg_client::api::{ContentSource, ContentSourceKind};
use warg_crypto::{
    hash::{DynHash, HashAlgorithm},
    signing,
};
use warg_protocol::{package, ProtoEnvelope, Version, registry::RecordId};

use crate::{advise_end_publish, advise_set_registry, advise_start_publish, data::CliData};

#[derive(Debug, Subcommand)]
pub enum PublishCommand {
    Start {
        #[clap(long)]
        name: String,
        #[clap(long)]
        init: bool,
    },
    Release {
        version: Version,
        #[clap(long)]
        path: PathBuf,
    },
    List,
    Abort,
    Submit,
}

pub async fn publish_command(
    data: CliData,
    signing_key: signing::PrivateKey,
    command: PublishCommand,
) -> Result<()> {
    let reg_info = match data.get_registry_info()? {
        Some(info) => info,
        None => {
            println!("Must have a registry set to publish.");
            advise_set_registry();
            return Err(Error::msg("must have a registry set to publish."));
        }
    };
    let pub_info = data.get_publish_info()?;

    match command {
        PublishCommand::Start { name, init } => {
            if let Some(_) = pub_info {
                eprintln!("Can't start a new publish.");
                advise_end_publish();
            } else {
                let prev = match init {
                    true => None,
                    false => Some(todo!()),
                };
                let mut info = PublishInfo::new(name, prev);
                info.push_init(HashAlgorithm::Sha256, signing_key.public_key());
                data.set_publish_info(&info)?;
            }
        }
        PublishCommand::Release { version, path } => {
            if let Some(mut info) = pub_info {
                let digest = data.add_content(&path)?;
                info.push_release(version, digest);
                data.set_publish_info(&info)?;
            } else {
                eprintln!("Cannot queue a release.");
                advise_start_publish();
            }
        }
        PublishCommand::List => {
            if let Some(info) = pub_info {
                println!("{}", info);
            } else {
                println!("No current publish to list.");
                advise_start_publish();
            }
        }
        PublishCommand::Abort => {
            data.clear_publish_info()?;
            println!("Publish aborted.");
        }
        PublishCommand::Submit => {
            if let Some(info) = pub_info {
                let (name, contents, record) = info.finalize();
                let record = ProtoEnvelope::signed_contents(&signing_key, record)?;

                let client = warg_client::api::Client::new(reg_info.url().to_owned());
                let mut content_sources = Vec::new();
                if !contents.is_empty() {
                    println!("Uploading contents");
                    for content in contents {
                        let content_path = data.content_path(&content);
                        let content_file = tokio::fs::File::open(content_path).await?;
                        client.upload_content(content_file).await?;

                        let url = format!(
                            "{}/content/{}",
                            reg_info.url(),
                            content.to_string().replace(":", "-")
                        );
                        println!("Expected url: {}", url);
                        content_sources.push(ContentSource {
                            content_digest: content,
                            kind: ContentSourceKind::HttpAnonymous { url },
                        })
                    }
                }
                print!("Submitting");
                let response = client
                    .publish(&name, Arc::new(record), content_sources)
                    .await?;
                println!("Response: {:#?}", response);
            } else {
                eprintln!("No publish to submit.");
                advise_start_publish();
            }
        }
    };

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct PublishInfo {
    package: String,
    prev: Option<RecordId>,
    entries: Vec<PackageEntryInfo>,
}

impl PublishInfo {
    pub fn new(package: String, prev: Option<RecordId>) -> Self {
        Self {
            package,
            prev,
            entries: vec![],
        }
    }

    pub fn push_init(&mut self, hash_algorithm: HashAlgorithm, key: signing::PublicKey) {
        let init = PackageEntryInfo::Init {
            hash_algorithm,
            key,
        };
        self.entries.push(init);
    }

    pub fn push_release(&mut self, version: Version, content: DynHash) {
        let release = PackageEntryInfo::Release { version, content };
        self.entries.push(release);
    }

    pub fn finalize(self) -> (String, Vec<DynHash>, package::PackageRecord) {
        let name = self.package;
        let content = self
            .entries
            .iter()
            .filter_map(|entry| match entry {
                PackageEntryInfo::Init { .. } => None,
                PackageEntryInfo::Release { content, .. } => Some(content.to_owned()),
            })
            .collect();
        let package = package::PackageRecord {
            prev: self.prev,
            version: package::PACKAGE_RECORD_VERSION,
            timestamp: SystemTime::now(),
            entries: self
                .entries
                .into_iter()
                .map(|entry| match entry {
                    PackageEntryInfo::Init {
                        hash_algorithm,
                        key,
                    } => package::PackageEntry::Init {
                        hash_algorithm,
                        key,
                    },
                    PackageEntryInfo::Release { version, content } => {
                        package::PackageEntry::Release { version, content }
                    }
                })
                .collect(),
        };
        (name, content, package)
    }
}

impl fmt::Display for PublishInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Publishing package: {} ({} entries)\n",
            self.package,
            self.entries.len()
        )?;
        if let Some(prev) = &self.prev {
            println!("(Previous record hash: {})", prev);
        } else {
            println!("(No previous record, this publish must init)");
        }
        for (i, entry) in self.entries.iter().enumerate() {
            write!(f, "{}. {}\n", i, &entry)?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub enum PackageEntryInfo {
    Init {
        hash_algorithm: HashAlgorithm,
        key: signing::PublicKey,
    },
    Release {
        version: Version,
        content: DynHash,
    },
}

impl fmt::Display for PackageEntryInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageEntryInfo::Init {
                hash_algorithm,
                key,
            } => {
                write!(f, "Init {} - {}", hash_algorithm, key)
            }
            PackageEntryInfo::Release { version, content } => {
                write!(f, "Release {} - {}", version, content)
            }
        }
    }
}
