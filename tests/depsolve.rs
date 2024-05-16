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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn depsolve() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;

    let client = create_client(&config).await?;
    let signing_key = support::test_signing_key();

    let mut head = publish_package(
        &client,
        &signing_key,
        "test:add",
        "tests/components/add.wat",
    )
    .await?;
    client
        .wait_for_publish(
            &PackageName::new("test:add")?,
            &head,
            Duration::from_millis(100),
        )
        .await?;
    head = publish_package(
        &client,
        &signing_key,
        "test:five",
        "tests/components/five.wat",
    )
    .await?;
    client
        .wait_for_publish(
            &PackageName::new("test:five")?,
            &head,
            Duration::from_millis(100),
        )
        .await?;
    head = publish_package(
        &client,
        &signing_key,
        "test:inc",
        "tests/components/inc.wat",
    )
    .await?;
    client
        .wait_for_publish(
            &PackageName::new("test:inc")?,
            &head,
            Duration::from_millis(100),
        )
        .await?;
    head = publish_package(
        &client,
        &signing_key,
        "test:meet",
        "tests/components/meet.wat",
    )
    .await?;
    client
        .wait_for_publish(
            &PackageName::new("test:meet")?,
            &head,
            Duration::from_millis(100),
        )
        .await?;

    client
        .fetch_packages([
            &PackageName::new("test:add")?,
            &PackageName::new("test:inc")?,
            &PackageName::new("test:five")?,
            &PackageName::new("test:meet")?,
        ])
        .await?;

    let info = client
        .registry()
        .load_package(
            client.get_warg_registry("test").await?.as_ref(),
            &PackageName::new("test:meet")?,
        )
        .await?
        .context("package does not exist in client storage")?;

    let locked_bytes = client.lock_component(&info).await?;
    let expected_locked = wat::parse_file("tests/components/meet_locked.wat")?;
    assert_eq!(
        wasmprinter::print_bytes(&locked_bytes)?,
        wasmprinter::print_bytes(expected_locked)?
    );
    let bundled_bytes = client.bundle_component(&info).await?;
    let expected_bundled = wat::parse_file("tests/components/meet_bundled.wat")?;
    assert_eq!(
        wasmprinter::print_bytes(bundled_bytes)?,
        wasmprinter::print_bytes(expected_bundled)?
    );
    Ok(())
}

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
