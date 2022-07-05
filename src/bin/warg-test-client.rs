use std::{path::PathBuf, process::exit};

use anyhow::{bail, Context};
use clap::{Args, Parser};
use tokio::{fs::File, io::AsyncSeekExt};
use tokio_util::compat::TokioAsyncReadCompatExt;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use wasm_registry::{
    client::Client,
    digest::{Sha256Digest, TypedDigest},
    release::{EntityName, EntityType, ReleaseManifest, RELEASE_PAYLOAD_TYPE},
};

#[derive(Parser)]
enum Command {
    Fetch(FetchCommand),
    Publish(PublishCommand),
}

impl Command {
    async fn run(self) -> anyhow::Result<()> {
        match self {
            Command::Fetch(subcmd) => subcmd.run().await,
            Command::Publish(subcmd) => subcmd.run().await,
        }
    }
}

#[derive(Args)]
struct ServerArgs {
    #[clap(long = "--server", default_value = "http://127.0.0.1:9999")]
    base_url: reqwest::Url,
}

#[derive(Args)]
struct FetchCommand {
    name: EntityName,
    version: semver::Version,

    #[clap(flatten)]
    server: ServerArgs,
}

impl FetchCommand {
    async fn run(self) -> anyhow::Result<()> {
        let client = Client::new(self.server.base_url);

        let release = client
            .get_release(EntityType::Component, self.name, self.version)
            .await
            .context("Failed to get release")?;

        println!(
            "Got release: {}\n",
            serde_json::to_string_pretty(&release).unwrap()
        );

        println!("Fetching content...");

        let content = client
            .fetch_validate_content(&release)
            .await
            .context("Failed to fetch content")?;

        println!("Successfully fetched and validated content!");

        println!("First 100 chars/bytes of content:");
        if let Ok(s) = std::str::from_utf8(&content) {
            println!("{}\n", &s[..100]);
        } else {
            println!("{:?}\n", &content[..100]);
        }

        Ok(())
    }
}

#[derive(Args)]
struct PublishCommand {
    name: EntityName,
    version: semver::Version,
    content: PathBuf,

    #[clap(flatten)]
    server: ServerArgs,
}

impl PublishCommand {
    async fn run(self) -> anyhow::Result<()> {
        let mut content = File::open(self.content)
            .await
            .context("Failed to open content file")?;

        let digest = Sha256Digest::digest_read((&mut content).compat())
            .await
            .context("Failed to calculate content digest")?;

        content
            .seek(std::io::SeekFrom::Start(0))
            .await
            .context("Failed to reset content file cursor")?;

        let release = ReleaseManifest {
            entity_type: EntityType::Component,
            name: self.name,
            version: self.version,
            content_digest: TypedDigest::Sha256(digest),
        };

        println!(
            "Prepared release manifest: {}\n",
            serde_json::to_string_pretty(&release).unwrap()
        );

        let client = Client::new(self.server.base_url);

        let (maintainer_key, secret_key) = client
            .register_generated_maintainer_key()
            .await
            .context("Failed to register maintainer key")?;

        println!("Registered maintainer key id={:?}\n", &maintainer_key.id);

        let unpublished = client
            .create_unpublished_release(&release)
            .await
            .context("Failed to create unpublished release")?;

        println!(
            "Created unpublished release: {}\n",
            serde_json::to_string_pretty(&unpublished).unwrap()
        );

        if let Some(upload_url) = unpublished.upload_url {
            client
                .upload_content(&upload_url, content)
                .await
                .context("Failed to upload content")?;

            println!("Uploaded content to {:?}\n", upload_url);
        } else {
            bail!("no upload_url");
        }

        let signature =
            secret_key.sign_payload(RELEASE_PAYLOAD_TYPE, unpublished.release.as_bytes())?;

        println!(
            "Prepared release signature: {}\n",
            serde_json::to_string_pretty(&signature).unwrap()
        );

        let published = client
            .publish(&release, signature)
            .await
            .context("Publish failed")?;

        println!(
            "Published release: {}",
            serde_json::to_string_pretty(&published).unwrap()
        );

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    if let Err(err) = Command::parse().run().await {
        eprintln!("Error: {:?}", err);
        exit(2);
    }
}
