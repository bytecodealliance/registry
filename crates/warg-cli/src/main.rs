mod publish_info;
mod registry_info;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Error, Result};
use clap::{Parser, Subcommand};
use publish_info::PublishInfo;
use registry_info::RegistryInfo;
use serde::{Deserialize, Serialize};
use warg_client::api::{ContentSource, ContentSourceKind};
use warg_crypto::{
    hash::{DynHash, Hash, HashAlgorithm, Sha256},
    signing,
};
use warg_protocol::{ProtoEnvelope, Version};

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    SetRegistry {
        registry: String,
    },
    Publish {
        #[command(subcommand)]
        subcommand: PublishCommand,
    },
}

#[derive(Debug, Subcommand)]
enum PublishCommand {
    Start {
        #[clap(long)]
        name: String,
        #[clap(long)]
        no_prev: bool,
    },
    Init,
    Release {
        version: Version,
        #[clap(long)]
        path: PathBuf,
    },
    List,
    Abort,
    Submit,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let args = Args::parse();

    let demo_user_key = std::env::var("WARG_DEMO_USER_KEY")?;
    let demo_user_key: signing::PrivateKey = demo_user_key.parse()?;

    let paths = CliPaths::new();

    match args.command {
        Commands::SetRegistry { registry } => {
            let reg_info = RegistryInfo::new(registry);
            store(paths.registry_info(), &reg_info)?;
            Ok(())
        }
        Commands::Publish { subcommand } => {
            let reg_info = load(paths.registry_info())?.unwrap();
            publish_command(paths, demo_user_key, reg_info, subcommand).await
        }
    }
}

async fn publish_command(
    paths: CliPaths,
    signing_key: signing::PrivateKey,
    reg_info: RegistryInfo,
    command: PublishCommand,
) -> Result<()> {
    let info: Option<PublishInfo> = load(paths.publish_info())?;

    match command {
        PublishCommand::Start { name, no_prev } => {
            if let Some(_) = info {
                eprintln!("Can't start a new publish.");
                advise_end_publish();
            } else {
                let prev = match no_prev {
                    true => None,
                    false => Some(todo!()),
                };
                let info = PublishInfo::new(name, prev);
                store(paths.publish_info(), &info)?;
            }
        }
        PublishCommand::Init => {
            if let Some(mut info) = info {
                info.push_init(HashAlgorithm::Sha256, signing_key.public_key());
                store(paths.publish_info(), &info)?;
            } else {
                eprintln!("Cannot queue an initialization.");
                advise_start_publish();
            }
        }
        PublishCommand::Release { version, path } => {
            if let Some(mut info) = info {
                let contents = fs::read_to_string(path)?;
                let digest: Hash<Sha256> = Hash::of(&contents.as_str());
                let digest: DynHash = digest.into();
                let content_path = paths.content(&digest);
                dump(&content_path, &contents)?;
                info.push_release(version, digest);
                store(paths.publish_info(), &info)?;
            } else {
                eprintln!("Cannot queue a release.");
                advise_start_publish();
            }
        }
        PublishCommand::List => {
            if let Some(info) = info {
                println!("{}", info);
            }
        }
        PublishCommand::Abort => {
            delete(paths.publish_info())?;
            println!("Publish aborted.");
        }
        PublishCommand::Submit => {
            if let Some(info) = info {
                let (name, contents, record) = info.finalize();
                let record = ProtoEnvelope::signed_contents(&signing_key, record)?;

                let client = warg_client::api::Client::new(reg_info.url().to_owned());
                let mut content_sources = Vec::new();
                if !contents.is_empty() {
                    println!("Uploading contents");
                    for content in contents {
                        let content_path = paths.content(&content);
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

fn advise_end_publish() {
    eprintln!("Warg must not be publishing already.");
    eprintln!("Use 'publish' or 'abort' to resolve this publish.");
}

fn advise_start_publish() {
    eprintln!("Warg must be in publishing mode.");
    eprintln!("Use 'create-package' or 'start-publish' to begin publishing.");
}

pub struct CliPaths {
    base: PathBuf,
    publish_info: PathBuf,
    registry_info: PathBuf,
}

impl CliPaths {
    pub fn new() -> Self {
        let mut base = PathBuf::from(".");
        base.push(".warg");
        let mut publish_info = base.clone();
        publish_info.push("publish-info.json");
        let mut registry_info = base.clone();
        registry_info.push("registry-info.json");
        Self {
            base,
            publish_info,
            registry_info,
        }
    }

    fn base(&self) -> &Path {
        &self.base
    }

    fn content(&self, digest: &DynHash) -> PathBuf {
        let sanitized = digest.to_string().replace(":", "-");

        let mut path = self.base().to_owned();
        path.push("content");
        path.push(sanitized);
        path
    }

    fn publish_info(&self) -> &Path {
        &self.publish_info
    }

    fn registry_info(&self) -> &Path {
        &self.registry_info
    }
}

fn load<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<Option<T>> {
    if path.is_file() {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|_| Error::msg(format!("Info at path {:?} is malformed", &path)))
    } else {
        Ok(None)
    }
}

fn store<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let contents = serde_json::to_string_pretty(value)?;
    dump(path, &contents)?;
    Ok(())
}

fn dump(path: &Path, contents: &str) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    if let Some(parent) = path.parent() {
        if parent.is_file() {
            return Err(Error::msg("Parent directory is file"));
        }
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, contents)?;
    Ok(())
}

fn delete(path: &Path) -> Result<()> {
    std::fs::remove_file(&path)?;
    Ok(())
}
