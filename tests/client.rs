use self::support::*;
use anyhow::{bail, Context, Result};
use std::fs;
use warg_client::{
    storage::{ContentStorage, PublishEntry, PublishInfo, RegistryStorage},
    Config, FileSystemClient, StorageLockResult,
};

pub mod support;

fn create_client(config: &Config) -> Result<FileSystemClient> {
    match FileSystemClient::try_new_with_config(None, config)? {
        StorageLockResult::Acquired(client) => Ok(client),
        _ => bail!("failed to acquire storage lock"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn client_incrementally_fetches() -> Result<()> {
    const PACKAGE_COUNT: usize = 2000;

    let (_server, config) = spawn_server(&root().await?, None).await?;
    let client = create_client(&config)?;
    let signing_key = support::test_signing_key().parse().unwrap();

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

    // Initialize a new package log with releases of the same content
    // As the number of packages exceeds the limit for fetching in a single request,
    // this will guarantee that the later call to `upsert` fetches the package log incrementally
    let mut entries = Vec::with_capacity(PACKAGE_COUNT + 1);
    entries.push(PublishEntry::Init);

    for i in 1..=PACKAGE_COUNT {
        entries.push(PublishEntry::Release {
            version: format!("0.{i}.0").parse().unwrap(),
            content: digest.clone(),
        });
    }

    client
        .publish_with_info(
            &signing_key,
            PublishInfo {
                package: "test:package".to_string(),
                entries,
            },
        )
        .await?;

    drop(client);

    // Delete the client's registry storage directory to ensure it fetches
    fs::remove_dir_all(config.registries_dir.as_ref().unwrap())
        .context("failed to remove registries directory")?;

    // Recreate the client with the same config
    let client = create_client(&config)?;

    // Fetch the package log
    client.upsert(&["test:package"]).await?;

    // Ensure the package log exists and has releases with all with the same digest
    let package = client
        .registry()
        .load_package("test:package")
        .await?
        .context("package does not exist in client storage")?;

    let mut count = 0;
    for release in package.state.releases() {
        assert_eq!(release.content(), Some(&digest));
        count += 1;
    }

    assert_eq!(count, PACKAGE_COUNT);

    Ok(())
}
