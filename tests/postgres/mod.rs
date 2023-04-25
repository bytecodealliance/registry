use std::path::Path;

use anyhow::{Context, Result};
use futures::Future;
use warg_client::{api, Config};
use warg_server::datastore::{DataStore, PostgresDataStore};

fn data_store() -> Result<Box<dyn DataStore>> {
    Ok(Box::new(PostgresDataStore::new(
        std::env::var("DATABASE_URL")
            .context("failed to get `DATABASE_URL` environment variable")?,
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
/// This is one large test currently because there can be only one core service
/// running at a time due to the checkpoint state it keeps.
///
/// In the future when the core server is extracted out of the server,
/// this test can then be broken apart into smaller tests.
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
        let response = client.latest_checkpoint().await?;
        assert_eq!(response.checkpoint.as_ref().log_length, 2);

        Ok(())
    })
    .await?;

    // Validate the component package is still present after a restart
    run(&root, |config| async move {
        let client = super::create_client(&config)?;
        super::validate_component_package(&config, &client).await?;

        // There should be two log entries in the registry
        let client = api::Client::new(config.default_url.as_ref().unwrap())?;
        let response = client.latest_checkpoint().await?;
        assert_eq!(response.checkpoint.as_ref().log_length, 2);

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
        let response = client.latest_checkpoint().await?;
        assert_eq!(response.checkpoint.as_ref().log_length, 3);

        Ok(())
    })
    .await?;

    // Validate the wit package is still present after a restart
    run(&root, |config| async move {
        let client = super::create_client(&config)?;
        super::validate_wit_package(&config, &client).await?;

        // There should be three log entries in the registry
        let client = api::Client::new(config.default_url.as_ref().unwrap())?;
        let response = client.latest_checkpoint().await?;
        assert_eq!(response.checkpoint.as_ref().log_length, 3);

        Ok(())
    })
    .await?;

    Ok(())
}
