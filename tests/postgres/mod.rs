//! Tests for the PostgreSQL storage backend.

use super::{support::*, *};
use anyhow::{Context, Result};
use testresult::TestResult;
use warg_client::api;
use warg_protocol::registry::RegistryLen;
use warg_server::datastore::{DataStore, PostgresDataStore};

fn data_store() -> Result<Box<dyn DataStore>> {
    Ok(Box::new(PostgresDataStore::new(
        std::env::var("WARG_DATABASE_URL")
            .context("failed to get `WARG_DATABASE_URL` environment variable")?
            .into(),
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
    test_package_yanking(&config).await?;
    test_wit_publishing(&config).await?;
    test_wasm_content_policy(&config).await?;
    test_unauthorized_signing_key(&config).await?;
    // This is tested below where a different server is used that
    // allows any signing key
    //test_unknown_signing_key(&config).await?;
    test_invalid_signature(&config).await?;
    test_fetch_package_names(&config).await?;
    test_get_ledger(&config).await?;

    let mut packages = vec![
        PackageName::new("test:component")?,
        PackageName::new("test:yankee")?,
        PackageName::new("test:wit-package")?,
        PackageName::new("test:unauthorized-key")?,
    ];

    // There should be two log entries in the registry
    let client = api::Client::new(config.home_url.as_ref().unwrap(), None)?;
    let ts_checkpoint = client.latest_checkpoint(None).await?;
    assert_eq!(
        ts_checkpoint.as_ref().checkpoint.log_length,
        packages.len() as RegistryLen + 2, /* publishes + initial checkpoint + yank */
        "expected {len} packages plus the initial checkpoint and yank",
        len = packages.len()
    );

    drop(server);

    // Restart the server and ensure the data is still there
    let (server, config) = spawn_server(&root, None, Some(data_store()?), None).await?;

    test_unknown_signing_key(&config).await?;

    packages.push(PackageName::new("test:unknown-key")?);

    let client = api::Client::new(config.home_url.as_ref().unwrap(), None)?;
    let ts_checkpoint = client.latest_checkpoint(None).await?;
    assert_eq!(
        ts_checkpoint.as_ref().checkpoint.log_length,
        packages.len() as RegistryLen + 2, /* publishes + initial checkpoint + yank*/
        "expected {len} packages plus the initial checkpoint and yank",
        len = packages.len()
    );

    // Delete the client cache to force a complete download of all packages below
    fs::remove_dir_all(root.join("content"))?;
    fs::remove_dir_all(root.join("registries"))?;

    let client = create_client(&config).await?;
    client.fetch_packages(packages.iter()).await?;

    // Finally, after a restart, ensure the packages can be downloaded
    for package in packages {
        if package.name() == "yankee" {
            continue;
        }
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
