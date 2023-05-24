use std::path::Path;

use anyhow::{Context, Result};
use futures::Future;
use warg_client::{api, Config};
use warg_server::datastore::{DataStore, PostgresDataStore};

fn data_store() -> Result<Box<dyn DataStore>> {
    Ok(Box::new(PostgresDataStore::new(
        std::env::var("WARG_DATABASE_URL")
            .context("failed to get `WARG_DATABASE_URL` environment variable")?,
    )?))
}

async fn run<F>(root: &Path, callback: impl FnOnce(Config) -> F) -> Result<()>
where
    F: Future<Output = Result<()>>,
{
    let (_server, config) = super::support::spawn_server(root, Some(data_store()?)).await?;
    callback(config).await
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
async fn test() -> Result<()> {
    let root = super::root().await?;

    // Each of these is run with their own server instance to ensure restart works

    // Start by validating the initial checkpoint
    run(&root, |config| async {
        super::validate_initial_checkpoint(config).await
    })
    .await?;

    // Publish and validate a component package
    run(&root, |config| async move {
        let client = super::create_client(&config)?;
        super::publish_component_package(&client).await?;
        super::validate_component_package(&config, &client).await?;

        // There should be two log entries in the registry
        let client = api::Client::new(config.default_url.as_ref().unwrap())?;
        let checkpoint = client.latest_checkpoint().await?;
        assert_eq!(checkpoint.as_ref().log_length, 2);

        Ok(())
    })
    .await?;

    // Validate the component package is still present after a restart
    run(&root, |config| async move {
        let client = super::create_client(&config)?;
        super::validate_component_package(&config, &client).await?;

        // There should be two log entries in the registry
        let client = api::Client::new(config.default_url.as_ref().unwrap())?;
        let checkpoint = client.latest_checkpoint().await?;
        assert_eq!(checkpoint.as_ref().log_length, 2);

        Ok(())
    })
    .await?;

    // Publish and validate a wit package
    run(&root, |config| async move {
        let client = super::create_client(&config)?;
        super::publish_wit_package(&client).await?;
        super::validate_wit_package(&config, &client).await?;

        // There should be three log entries in the registry
        let client = api::Client::new(config.default_url.as_ref().unwrap())?;
        let checkpoint = client.latest_checkpoint().await?;
        assert_eq!(checkpoint.as_ref().log_length, 3);

        Ok(())
    })
    .await?;

    // Validate the wit package is still present after a restart
    run(&root, |config| async move {
        let client = super::create_client(&config)?;
        super::validate_wit_package(&config, &client).await?;

        // There should be three log entries in the registry
        let client = api::Client::new(config.default_url.as_ref().unwrap())?;
        let checkpoint = client.latest_checkpoint().await?;
        assert_eq!(checkpoint.as_ref().log_length, 3);

        Ok(())
    })
    .await?;

    Ok(())
}
