use self::support::*;
use anyhow::{bail, Context, Result};
use std::{fs, str::FromStr};
use warg_client::{api, Config, FileSystemClient, StorageLockResult};
use warg_crypto::{signing::PrivateKey, Encode, Signable};
use wit_component::DecodedWasm;
mod support;

#[cfg(feature = "postgres")]
mod postgres;

fn create_client(config: &Config) -> Result<FileSystemClient> {
    match FileSystemClient::try_new_with_config(None, config)? {
        StorageLockResult::Acquired(client) => Ok(client),
        _ => bail!("failed to acquire storage lock"),
    }
}

async fn validate_initial_checkpoint(config: Config) -> Result<()> {
    let client = api::Client::new(config.default_url.as_ref().unwrap())?;

    let response = client.latest_checkpoint().await?;

    // There should be only a single log entry (the initial operator log entry)
    // As the log leaf differs every time because it contains a timestamp,
    // the log root and map root can't be compared to a baseline value.
    assert_eq!(response.checkpoint.as_ref().log_length, 1);

    // Ensure the response was signed with the operator key
    let operator_key = PrivateKey::from_str(test_operator_key())?;
    assert_eq!(
        response.checkpoint.key_id().to_string(),
        operator_key.public_key().fingerprint().to_string()
    );

    // Ensure the signature matches the response
    warg_protocol::registry::MapCheckpoint::verify(
        &operator_key.public_key(),
        &response.checkpoint.as_ref().encode(),
        response.checkpoint.signature(),
    )?;

    Ok(())
}

async fn publish_component_package(client: &FileSystemClient) -> Result<()> {
    publish_component(client, "component:foo", "0.1.0", "(component)", true).await
}

async fn validate_component_package(config: &Config, client: &FileSystemClient) -> Result<()> {
    // Assert that the package can be downloaded
    client.upsert(&["component:foo"]).await?;
    let download = client
        .download("component:foo", &"0.1.0".parse()?)
        .await?
        .context("failed to resolve package")?;
    assert_eq!(
        download.digest.to_string(),
        "sha256:396bf81fe30c615180c31fc3ba964396241af472ace265f55609a3fcf12140f2"
    );
    assert_eq!(download.version, "0.1.0".parse()?);
    assert_eq!(
        download.path,
        config
            .content_dir
            .as_ref()
            .unwrap()
            .join("sha256")
            .join("396bf81fe30c615180c31fc3ba964396241af472ace265f55609a3fcf12140f2")
    );

    // Assert that it is a valid component
    match wit_component::decode(
        "foo",
        &fs::read(download.path).context("failed to read component")?,
    )? {
        DecodedWasm::Component(..) => {}
        _ => panic!("expected component"),
    }

    // Assert that a different version can't be downloaded
    assert!(client
        .download("component:foo", &"0.2.0".parse()?)
        .await?
        .is_none());

    Ok(())
}

async fn publish_wit_package(client: &FileSystemClient) -> Result<()> {
    publish_wit(client, "wit:foo", "0.1.0", "default world foo {}", true).await
}

async fn validate_wit_package(config: &Config, client: &FileSystemClient) -> Result<()> {
    // Assert that the package can be downloaded
    client.upsert(&["wit:foo"]).await?;
    let download = client
        .download("wit:foo", &"0.1.0".parse()?)
        .await?
        .context("failed to resolve package")?;
    assert_eq!(
        download.digest.to_string(),
        "sha256:eb83fbde29872c3c2da5a8485c60236b7a1ccaa461504cfb2ed52a6e9d9b2cfd"
    );
    assert_eq!(download.version, "0.1.0".parse()?);
    assert_eq!(
        download.path,
        config
            .content_dir
            .as_ref()
            .unwrap()
            .join("sha256")
            .join("eb83fbde29872c3c2da5a8485c60236b7a1ccaa461504cfb2ed52a6e9d9b2cfd")
    );

    // Assert it is a valid wit package
    match wit_component::decode(
        "foo",
        &fs::read(download.path).context("failed to read WIT package")?,
    )? {
        DecodedWasm::WitPackage(..) => {}
        _ => panic!("expected WIT package"),
    }

    // Assert that a different version can't be downloaded
    assert!(client
        .download("wit:foo", &"0.2.0".parse()?)
        .await?
        .is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_starts_with_initial_checkpoint() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None).await?;
    validate_initial_checkpoint(config).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_publishes_a_component() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None).await?;
    let client = create_client(&config)?;

    publish_component_package(&client).await?;
    validate_component_package(&config, &client).await?;

    // There should be two log entries in the registry
    let client = api::Client::new(config.default_url.as_ref().unwrap())?;
    let response = client.latest_checkpoint().await?;
    assert_eq!(response.checkpoint.as_ref().log_length, 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_publishes_a_wit_package() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None).await?;
    let client = create_client(&config)?;

    publish_wit_package(&client).await?;
    validate_wit_package(&config, &client).await?;

    // There should be two log entries in the registry
    let client = api::Client::new(config.default_url.as_ref().unwrap())?;
    let response = client.latest_checkpoint().await?;
    assert_eq!(response.checkpoint.as_ref().log_length, 2);

    Ok(())
}
