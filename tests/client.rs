use self::support::*;
use anyhow::{bail, Context, Result};
use std::{fs, time::Duration};
use warg_client::{
    storage::{ContentStorage, PublishEntry, PublishInfo, RegistryStorage},
    Config, FileSystemClient, StorageLockResult,
};
use warg_crypto::hash::{Hash, Sha256};

pub mod support;

fn create_client(config: &Config) -> Result<FileSystemClient> {
    match FileSystemClient::try_new_with_config(None, config)? {
        StorageLockResult::Acquired(client) => Ok(client),
        _ => bail!("failed to acquire storage lock"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn client_incrementally_fetches() -> Result<()> {
    const RELEASE_COUNT: usize = 300;
    const PACKAGE_NAME: &str = "test:package";

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
    dbg!("FIRST AWAIT");

    // Here we don't wait for a single publish operation to complete, except for the last one
    // If the last one is accepted, it implies that all the previous ones were accepted as well
    let mut head = client
        .publish_with_info(
            &signing_key,
            PublishInfo {
                package: PACKAGE_NAME.to_string(),
                head: None,
                entries: vec![PublishEntry::Init],
            },
        )
        .await?;

    dbg!("SECOND AWAIT");
    for i in 1..=RELEASE_COUNT {
        head = client
            .publish_with_info(
                &signing_key,
                PublishInfo {
                    package: PACKAGE_NAME.to_string(),
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
        .wait_for_publish(PACKAGE_NAME, &head, Duration::from_millis(100))
        .await?;
    dbg!("THIRD");
    drop(client);

    // Delete the client's registry storage directory to ensure it fetches
    fs::remove_dir_all(config.registries_dir.as_ref().unwrap())
        .context("failed to remove registries directory")?;

    // Recreate the client with the same config
    let client = create_client(&config)?;

    // Fetch the package log
    dbg!("BEFORE UPSERT");
    dbg!(Hash::<Sha256>::of(PACKAGE_NAME));
    client.upsert(&[PACKAGE_NAME]).await?;
    dbg!("FOURTH");

    // Ensure the package log exists and has releases with all with the same digest
    let package = client
        .registry()
        .load_package(PACKAGE_NAME)
        .await?
        .context("package does not exist in client storage")?;

    let mut count = 0;
    for release in package.state.releases() {
        assert_eq!(release.content(), Some(&digest));
        count += 1;
    }

    assert_eq!(count, RELEASE_COUNT);

    Ok(())
}
