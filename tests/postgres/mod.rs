//! Tests for the PostgreSQL storage backend.

use super::{support::*, *};
use anyhow::{Context, Result};
use testresult::TestResult;
use warg_client::api;
use warg_server::datastore::{DataStore, PostgresDataStore};

fn data_store() -> Result<Box<dyn DataStore>> {
    Ok(Box::new(PostgresDataStore::new(
        std::env::var("WARG_DATABASE_URL")
            .context("failed to get `WARG_DATABASE_URL` environment variable")?,
    )?))
}

/// This test assumes the database is empty on each run.
/// Use the `ci/run-postgres-tests.sh` script to run this test.
///
/// A smoke test that ensures that postgres integration works.
///
/// As the tests currently share the same database instance, we can only have one
/// warg server running at a time. This is because the server keeps checkpoint
/// data structures in memory and assumes that it receives all log leafs processed
/// by the server.
///
/// In the future, when either the tests have completely isolated databases or
/// the checkpointing service is moved to a separate process, we can extract this
/// out to multiple tests.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn it_works_with_postgres() -> TestResult {
    let root = root().await?;
    let (server, config) = spawn_server(
        &root,
        None,
        Some(data_store()?),
        Some(vec![(
            "test".to_string(),
            test_signing_key().public_key().fingerprint(),
        )]),
    )
    .await?;

    // This should be the same set of tests as in `tests/memory/mod.rs`
    test_initial_checkpoint(&config).await?;
    test_component_publishing(&config).await?;
    test_wit_publishing(&config).await?;
    test_wasm_content_policy(&config).await?;
    test_unauthorized_signing_key(&config).await?;
    // This is tested below where a different server is used that
    // allows any signing key
    //test_unknown_signing_key(&config).await?;
    test_invalid_signature(&config).await?;

    let mut packages = vec![
        PackageId::new("test:component")?,
        PackageId::new("test:wit-package")?,
        PackageId::new("test:unauthorized-key")?,
    ];

    // There should be two log entries in the registry
    let client = api::Client::new(config.default_url.as_ref().unwrap())?;
    let checkpoint = client.latest_checkpoint().await?;
    assert_eq!(
        checkpoint.as_ref().log_length,
        packages.len() as u32 + 1, /* initial checkpoint */
        "expected {len} packages plus the initial checkpoint",
        len = packages.len()
    );

    drop(server);

    // Restart the server and ensure the data is still there
    let (server, config) = spawn_server(&root, None, Some(data_store()?), None).await?;

    test_unknown_signing_key(&config).await?;

    packages.push(PackageId::new("test:unknown-key")?);

    let client = api::Client::new(config.default_url.as_ref().unwrap())?;
    let checkpoint = client.latest_checkpoint().await?;
    assert_eq!(
        checkpoint.as_ref().log_length,
        packages.len() as u32 + 1, /* initial checkpoint */
        "expected {len} packages plus the initial checkpoint",
        len = packages.len()
    );

    // Delete the client cache to force a complete download of all packages below
    fs::remove_dir_all(root.join("content"))?;
    fs::remove_dir_all(root.join("registries"))?;

    let client = create_client(&config)?;
    client.upsert(packages.iter()).await?;

    // Finally, after a restart, ensure the packages can be downloaded
    for package in packages {
        client
            .download(&package, &"0.1.0".parse()?)
            .await?
            .context("failed to resolve package")?;
    }

    // Restart the server for the custom content URL test
    drop(client);
    drop(server);
    let (_server, config) = spawn_server(
        &root,
        Some("https://example.com".parse().unwrap()),
        Some(data_store()?),
        None,
    )
    .await?;

    test_custom_content_url(&config).await?;

    Ok(())
}
