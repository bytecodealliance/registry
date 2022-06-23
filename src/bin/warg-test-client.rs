use std::{path::PathBuf, process::exit};

use anyhow::{bail, Context};
use clap::{Args, Parser};
use tokio::{fs::File, io::AsyncSeekExt};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use wasm_registry::{
    client::Client,
    digest::{Sha256Digest, TypedDigest},
    dsse::Signature,
    release::{EntityName, EntityType, ReleaseManifest, RELEASE_PAYLOAD_TYPE},
};

#[derive(Parser)]
enum Command {
    Publish(PublishCommand),
}

impl Command {
    async fn run(self) -> anyhow::Result<()> {
        match self {
            Command::Publish(subcmd) => subcmd.run(),
        }
        .await
    }
}

#[derive(Args)]
struct ServerArgs {
    #[clap(long = "--server", default_value = "http://127.0.0.1:9999")]
    base_url: reqwest::Url,
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

        let digest = Sha256Digest::digest_read(&mut content)
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

        let (publisher, secret_key) = client
            .register_publisher()
            .await
            .context("Failed to register publisher")?;
        let publisher_id = publisher.id.unwrap();

        println!("Registered publisher id={:?}\n", &publisher_id);

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

        let signature = Signature::sign(
            RELEASE_PAYLOAD_TYPE,
            unpublished.release.as_bytes(),
            publisher_id,
            secret_key,
        )?;

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
