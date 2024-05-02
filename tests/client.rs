use self::support::*;
use anyhow::{bail, Context, Result};
use std::{fs, time::Duration};
use warg_client::{
    storage::{ContentStorage, PublishEntry, PublishInfo, RegistryStorage},
    Config, FileSystemClient, StorageLockResult,
};
use warg_protocol::registry::PackageName;

pub mod support;

fn create_client(config: &Config) -> Result<FileSystemClient> {
    match FileSystemClient::try_new_with_config(None, config, None)? {
        StorageLockResult::Acquired(client) => Ok(client),
        _ => bail!("failed to acquire storage lock"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn client_incrementally_fetches() -> Result<()> {
    const RELEASE_COUNT: usize = 300;
    const PACKAGE_NAME: &str = "test:package";

    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;

    let client = create_client(&config)?;
    let signing_key = support::test_signing_key();

    // Store a single component that will be used for every release
    let bytes =
        wat::parse_str("(component)").context("failed to parse component for publishing")?;
    let digest = client
        .content()
        .store_content(
            Box::pin(futures::stream::once(async move { Ok(bytes.into()) })),
            None,
        )
        .await?;

    let name = PackageName::new(PACKAGE_NAME)?;

    let mut head = client
        .publish_with_info(
            &signing_key,
            PublishInfo {
                name: name.clone(),
                head: None,
                entries: vec![PublishEntry::Init],
            },
        )
        .await?;

    client
        .wait_for_publish(&name, &head, Duration::from_millis(100))
        .await?;

    // Here we don't wait for a single publish operation to complete, except for the last one
    // If the last one is accepted, it implies that all the previous ones were accepted as well
    for i in 1..=RELEASE_COUNT {
        head = client
            .publish_with_info(
                &signing_key,
                PublishInfo {
                    name: name.clone(),
                    head: Some(head),
                    entries: vec![PublishEntry::Release {
                        version: format!("0.{i}.0").parse().unwrap(),
                        content: digest.clone(),
                    }],
                },
            )
            .await?;
    }

    client
        .wait_for_publish(&name, &head, Duration::from_millis(100))
        .await?;

    drop(client);

    // Delete the client's registry storage directory to ensure it fetches
    fs::remove_dir_all(config.registries_dir.as_ref().unwrap())
        .context("failed to remove registries directory")?;

    // Recreate the client with the same config
    let client = create_client(&config)?;

    // Regression test: update on empty registry storage
    client.update().await?;

    // Fetch the package log
    let package = client.package(&name).await?;

    // Ensure the package log exists and has releases with all with the same digest
    assert_eq!(
        package.name,
        client
            .registry()
            .load_package(
                client.get_warg_registry(name.namespace()).await?.as_ref(),
                &name,
            )
            .await?
            .context("package does not exist in client storage")?
            .name
    );

    let mut count = 0;
    for release in package.state.releases() {
        assert_eq!(release.content(), Some(&digest));
        count += 1;
    }

    assert_eq!(count, RELEASE_COUNT);

    Ok(())
}
