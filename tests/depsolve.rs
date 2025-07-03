use self::support::*;
use anyhow::{Context, Result};
use std::time::Duration;
use warg_client::{
    storage::{
        ContentStorage, FileSystemContentStorage, FileSystemNamespaceMapStorage,
        FileSystemRegistryStorage, PublishEntry, PublishInfo, RegistryStorage,
    },
    Client,
};
use warg_crypto::signing::PrivateKey;
use warg_protocol::registry::{PackageName, RecordId};

pub mod support;

async fn publish_package(
    client: &Client<
        FileSystemRegistryStorage,
        FileSystemContentStorage,
        FileSystemNamespaceMapStorage,
    >,
    signing_key: &PrivateKey,
    name: &str,
    path: &str,
) -> Result<RecordId> {
    let comp = wat::parse_file(path)?;
    let name = PackageName::new(name)?;
    let add_digest = client
        .content()
        .store_content(
            Box::pin(futures::stream::once(async move { Ok(comp.into()) })),
            None,
        )
        .await?;
    let mut head = client
        .publish_with_info(
            signing_key,
            PublishInfo {
                name: name.clone(),
                head: None,
                entries: vec![PublishEntry::Init],
            },
        )
        .await?;
    client
        .wait_for_publish(&name.clone(), &head, Duration::from_millis(100))
        .await?;
    head = client
        .publish_with_info(
            signing_key,
            PublishInfo {
                name: name.clone(),
                head: Some(head),
                entries: vec![PublishEntry::Release {
                    version: "1.0.0".to_string().parse().unwrap(),
                    content: add_digest.clone(),
                }],
            },
        )
        .await?;
    Ok(head)
}
