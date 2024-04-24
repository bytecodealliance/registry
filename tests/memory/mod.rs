//! Tests for the in-memory storage backend.

use super::{support::*, *};
use anyhow::Result;
use warg_client::api;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_starts_with_initial_checkpoint() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;
    test_initial_checkpoint(&config).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_publishes_a_component() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;
    test_component_publishing(&config).await?;

    // There should be two log entries in the registry
    let client = api::Client::new(config.home_url.as_ref().unwrap(), None)?;
    let ts_checkpoint = client.latest_checkpoint().await?;
    assert_eq!(
        ts_checkpoint.as_ref().checkpoint.log_length,
        2,
        "expected two log entries (initial + component)"
    );

    test_fetch_package_names(&config).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_yanks_a_package() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;
    test_package_yanking(&config).await?;

    // There should be three entries in the registry
    let client = api::Client::new(config.home_url.as_ref().unwrap(), None)?;
    let ts_checkpoint = client.latest_checkpoint().await?;
    assert_eq!(
        ts_checkpoint.as_ref().checkpoint.log_length,
        3,
        "expected three log entries (initial + release + yank)"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_publishes_a_wit_package() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;
    test_wit_publishing(&config).await?;

    // There should be two log entries in the registry
    let client = api::Client::new(config.home_url.as_ref().unwrap(), None)?;
    let ts_checkpoint = client.latest_checkpoint().await?;
    assert_eq!(
        ts_checkpoint.as_ref().checkpoint.log_length,
        2,
        "expected two log entries (initial + wit)"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_rejects_non_wasm_content() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;
    test_wasm_content_policy(&config).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_rejects_unauthorized_signing_key() -> Result<()> {
    let (_server, config) = spawn_server(
        &root().await?,
        None,
        None,
        Some(vec![(
            "test".to_string(),
            test_signing_key().public_key().fingerprint(),
        )]),
    )
    .await?;

    test_unauthorized_signing_key(&config).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_rejects_unknown_signing_key() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;
    test_unknown_signing_key(&config).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_rejects_invalid_signature() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;
    test_invalid_signature(&config).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_formats_custom_content_urls() -> Result<()> {
    let (_server, config) = spawn_server(
        &root().await?,
        Some("https://example.com".parse().unwrap()),
        None,
        None,
    )
    .await?;
    test_custom_content_url(&config).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_get_ledger() -> Result<()> {
    let (_server, config) = spawn_server(&root().await?, None, None, None).await?;
    test_get_ledger(&config).await
}
