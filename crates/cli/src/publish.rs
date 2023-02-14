use anyhow::{Context, Result};
use warg_client::{Client, ClientError, PackageEntryInfo, PublishInfo};
use warg_crypto::signing;

use crate::PublishCommand;

pub async fn publish_command(
    mut client: Client,
    signing_key: signing::PrivateKey,
    command: PublishCommand,
) -> Result<(), ClientError> {
    match command {
        PublishCommand::Start { name, init } => {
            if init {
                client
                    .start_publish_init(name, signing_key.public_key())
                    .await
            } else {
                client.start_publish(name).await
            }
        }
        PublishCommand::Release { version, path } => {
            let file_content = tokio::fs::read(path)
                .await
                .with_context(|| "Unable to read specified content.")?;
            let mut new_content = client.storage().create_content().await?;
            new_content.write_all(file_content.as_slice()).await?;
            let digest = new_content.finalize().await?;

            client.queue_release(version, digest).await
        }
        PublishCommand::List => {
            if let Some(info) = client.storage().load_publish_info().await? {
                print_publish_info(&info);
                Ok(())
            } else {
                Err(ClientError::NotPublishing)
            }
        }
        PublishCommand::Abort => client.cancel_publish().await,
        PublishCommand::Submit => client.submit_publish(&signing_key).await,
    }
}

fn print_publish_info(info: &PublishInfo) {
    println!(
        "Publishing package: {} ({} entries)\n",
        info.package,
        info.entries.len()
    );
    if let Some(prev) = &info.prev {
        println!("(Previous record hash: {})", prev);
    } else {
        println!("(No previous record, this publish must init)");
    }
    for (i, entry) in info.entries.iter().enumerate() {
        print!("{}", i);
        print_package_entry_info(entry)
    }
}

fn print_package_entry_info(info: &PackageEntryInfo) {
    match info {
        PackageEntryInfo::Init {
            hash_algorithm,
            key,
        } => {
            println!("Init {} - {}", hash_algorithm, key)
        }
        PackageEntryInfo::Release { version, content } => {
            println!("Release {} - {}", version, content)
        }
    }
}
