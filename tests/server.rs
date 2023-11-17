use self::support::*;
use anyhow::{Context, Result};
use rand_core::OsRng;
use reqwest::StatusCode;
use std::{
    borrow::Cow,
    fs,
    time::{Duration, SystemTime},
};
use url::Url;
use warg_api::v1::{
    content::{ContentSource, ContentSourcesResponse},
    fetch::{FetchPackageNamesRequest, FetchPackageNamesResponse},
    ledger::{LedgerSource, LedgerSourceContentType, LedgerSourcesResponse},
    package::PublishRecordRequest,
    paths,
};
use warg_client::{
    api,
    storage::{PublishEntry, PublishInfo, RegistryStorage},
    ClientError, Config,
};
use warg_crypto::{
    hash::{HashAlgorithm, Sha256},
    signing::PrivateKey,
    Encode, Signable,
};
use warg_protocol::{
    package::{PackageEntry, PackageRecord, PACKAGE_RECORD_VERSION},
    registry::{LogId, PackageName},
    ProtoEnvelope, ProtoEnvelopeBody, Version,
};
use wit_component::DecodedWasm;

mod support;

mod memory;
#[cfg(feature = "postgres")]
mod postgres;

async fn test_initial_checkpoint(config: &Config) -> Result<()> {
    let client = api::Client::new(config.default_url.as_ref().unwrap())?;

    let ts_checkpoint = client.latest_checkpoint().await?;
    let checkpoint = &ts_checkpoint.as_ref().checkpoint;

    // There should be only a single log entry (the initial operator log entry)
    // As the log leaf differs every time because it contains a timestamp,
    // the log root and map root can't be compared to a baseline value.
    assert_eq!(checkpoint.log_length, 1);

    // Ensure the response was signed with the operator key
    let operator_key = test_operator_key();
    assert_eq!(
        ts_checkpoint.key_id().to_string(),
        operator_key.public_key().fingerprint().to_string()
    );

    // Ensure the signature matches the response
    warg_protocol::registry::TimestampedCheckpoint::verify(
        &operator_key.public_key(),
        &ts_checkpoint.as_ref().encode(),
        ts_checkpoint.signature(),
    )?;

    Ok(())
}

async fn test_component_publishing(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:component";
    const PACKAGE_VERSION: &str = "0.1.0";

    let name = PackageName::new(PACKAGE_NAME)?;
    let client = create_client(config)?;
    let signing_key = test_signing_key();
    let digest = publish_component(
        &client,
        &name,
        PACKAGE_VERSION,
        "(component)",
        true,
        &signing_key,
    )
    .await?;

    // Assert that the package can be downloaded
    client.upsert([&name]).await?;
    let download = client
        .download(&name, &PACKAGE_VERSION.parse()?)
        .await?
        .context("failed to resolve package")?;

    assert_eq!(download.digest, digest);
    assert_eq!(download.version, PACKAGE_VERSION.parse()?);
    assert_eq!(
        download.path,
        config
            .content_dir
            .as_ref()
            .unwrap()
            .join("sha256")
            .join(download.digest.to_string().strip_prefix("sha256:").unwrap())
    );

    // Assert that it is a valid component
    match wit_component::decode(&fs::read(download.path).context("failed to read component")?)? {
        DecodedWasm::Component(..) => {}
        _ => panic!("expected component"),
    }

    // Assert that a different version can't be downloaded
    assert!(client.download(&name, &"0.2.0".parse()?).await?.is_none());

    Ok(())
}

async fn test_package_yanking(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:yankee";
    const PACKAGE_VERSION: &str = "0.1.0";

    // Publish release
    let name = PackageName::new(PACKAGE_NAME)?;
    let client = create_client(config)?;
    let signing_key = test_signing_key();
    publish(
        &client,
        &name,
        PACKAGE_VERSION,
        wat::parse_str("(component)")?,
        true,
        &signing_key,
    )
    .await?;

    // Yank release
    let record_id = client
        .publish_with_info(
            &signing_key,
            PublishInfo {
                name: name.clone(),
                head: None,
                entries: vec![PublishEntry::Yank {
                    version: PACKAGE_VERSION.parse()?,
                }],
            },
        )
        .await?;
    client
        .wait_for_publish(&name, &record_id, Duration::from_millis(100))
        .await?;

    // Assert that the package is yanked
    client.upsert([&name]).await?;
    let opt = client.download(&name, &PACKAGE_VERSION.parse()?).await?;
    assert!(opt.is_none(), "expected no download, got {opt:?}");
    Ok(())
}

async fn test_wit_publishing(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:wit-package";
    const PACKAGE_VERSION: &str = "0.1.0";

    let name = PackageName::new(PACKAGE_NAME)?;
    let client = create_client(config)?;
    let signing_key = test_signing_key();
    let digest = publish_wit(
        &client,
        &name,
        PACKAGE_VERSION,
        &format!("package {PACKAGE_NAME}\nworld foo {{}}"),
        true,
        &signing_key,
    )
    .await?;

    // Assert that the package can be downloaded
    client.upsert([&name]).await?;
    let download = client
        .download(&name, &PACKAGE_VERSION.parse()?)
        .await?
        .context("failed to resolve package")?;

    assert_eq!(download.digest, digest);
    assert_eq!(download.version, PACKAGE_VERSION.parse()?);
    assert_eq!(
        download.path,
        config
            .content_dir
            .as_ref()
            .unwrap()
            .join("sha256")
            .join(download.digest.to_string().strip_prefix("sha256:").unwrap())
    );

    // Assert that it is a valid wit package
    match wit_component::decode(&fs::read(download.path).context("failed to read component")?)? {
        DecodedWasm::WitPackage(..) => {}
        _ => panic!("expected wit package"),
    }

    // Assert that a different version can't be downloaded
    assert!(client.download(&name, &"0.2.0".parse()?).await?.is_none());

    Ok(())
}

async fn test_wasm_content_policy(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:bad-content";
    const PACKAGE_VERSION: &str = "0.1.0";

    // Publish empty content to the server
    // This should be rejected by policy because it is not valid WebAssembly
    let name = PackageName::new(PACKAGE_NAME)?;
    let client = create_client(config)?;
    let signing_key = test_signing_key();
    match publish(
        &client,
        &name,
        PACKAGE_VERSION,
        Vec::new(),
        true,
        &signing_key,
    )
    .await
    .expect_err("expected publish to fail")
    .downcast::<ClientError>()
    {
        Ok(ClientError::PublishRejected {
            name: rejected_name,
            record_id,
            reason,
        }) => {
            assert_eq!(name, rejected_name);
            assert_eq!(
                reason,
                "content is not valid WebAssembly: unexpected end-of-file (at offset 0x0)"
            );

            // Waiting on the publish should fail with a rejection as well
            match client
                .wait_for_publish(&name, &record_id, Duration::from_millis(100))
                .await
                .expect_err("expected wait for publish to fail")
            {
                ClientError::PublishRejected {
                    name: rejected_name,
                    record_id: other,
                    reason,
                } => {
                    assert_eq!(name, rejected_name);
                    assert_eq!(record_id, other);
                    assert_eq!(
                        reason,
                        "content with digest `sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` was rejected by policy: content is not valid WebAssembly: unexpected end-of-file (at offset 0x0)"
                    );
                }
                _ => panic!("expected a content policy rejection error"),
            }
        }
        _ => panic!("expected a content policy rejection error"),
    }

    Ok(())
}

async fn test_unauthorized_signing_key(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:unauthorized-key";
    const PACKAGE_VERSION: &str = "0.1.0";

    // Start by publishing a new component package
    let name = PackageName::new(PACKAGE_NAME)?;
    let client = create_client(config)?;
    let signing_key = test_signing_key();
    publish_component(
        &client,
        &name,
        PACKAGE_VERSION,
        "(component)",
        true,
        &signing_key,
    )
    .await?;

    // Next, we're going to publish a new record signed by a different key
    let signing_key = PrivateKey::from(p256::ecdsa::SigningKey::random(&mut OsRng));

    let message = format!(
        "{:#}",
        publish_component(&client, &name, "0.2.0", "(component)", false, &signing_key,)
            .await
            .expect_err("expected publish to fail")
    );

    assert!(
        message.contains("not authorized to publish to package `test:unauthorized-key`"),
        "unexpected error message: {message}"
    );

    Ok(())
}

async fn test_unknown_signing_key(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:unknown-key";
    const PACKAGE_VERSION: &str = "0.1.0";

    // Start by publishing a new component package
    let name = PackageName::new(PACKAGE_NAME)?;
    let client = create_client(config)?;
    let signing_key = test_signing_key();
    publish_component(
        &client,
        &name,
        PACKAGE_VERSION,
        "(component)",
        true,
        &signing_key,
    )
    .await?;

    // Next, we're going to publish a new record signed by a different key
    // The new key is not currently known to the package log.
    let signing_key = PrivateKey::from(p256::ecdsa::SigningKey::random(&mut OsRng));

    let message = format!(
        "{:#}",
        publish_component(&client, &name, "0.2.0", "(component)", false, &signing_key,)
            .await
            .expect_err("expected publish to fail")
    );

    assert!(
        message.contains("unknown key id"),
        "unexpected error message: {message}"
    );

    Ok(())
}

async fn test_publishing_name_conflict(config: &Config) -> Result<()> {
    let client = create_client(config)?;
    let signing_key = test_signing_key();

    publish_component(
        &client,
        &PackageName::new("test:name")?,
        "0.1.0",
        "(component)",
        true,
        &signing_key,
    )
    .await?;

    // should be rejected
    publish_component(
        &client,
        &PackageName::new("test:NAME")?,
        "0.1.1",
        "(component)",
        true,
        &signing_key,
    )
    .await
    .expect_err("expected publish to fail");

    Ok(())
}

async fn test_invalid_signature(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:invalid-signature";

    // Use a reqwest client directly here as we're going to be sending an invalid signature
    let name = PackageName::new(PACKAGE_NAME)?;
    let log_id = LogId::package_log::<Sha256>(&name);
    let url = Url::parse(config.default_url.as_ref().unwrap())?
        .join(&paths::publish_package_record(&log_id))
        .unwrap();

    let signing_key = test_signing_key();
    let record = ProtoEnvelope::signed_contents(
        &signing_key,
        PackageRecord {
            prev: None,
            version: PACKAGE_RECORD_VERSION,
            timestamp: SystemTime::now(),
            entries: vec![PackageEntry::Init {
                hash_algorithm: warg_crypto::hash::HashAlgorithm::Sha256,
                key: signing_key.public_key(),
            }],
        },
    )?;

    let body = PublishRecordRequest {
        package_name: Cow::Borrowed(&name),
        record: Cow::Owned(ProtoEnvelopeBody::from(record)),
        content_sources: Default::default(),
    };

    // Update the signature to one that does not match the contents
    let mut body = serde_json::to_value(&body).unwrap();
    body["record"]["signature"] = serde_json::Value::String("ecdsa-p256:MEUCIQCzWZBW6ux9LecP66Y+hjmLZTP/hZVz7puzlPTXcRT2wwIgQZO7nxP0nugtw18MwHZ26ROFWcJmgCtKOguK031Y1D0=".to_string());

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .json(&serde_json::to_value(&body).unwrap())
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "unexpected response from server: {status}\n{body}",
    );
    assert!(
        body.contains("verification failed"),
        "unexpected response body: {body}"
    );

    Ok(())
}

async fn test_custom_content_url(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:custom-content-url";
    const PACKAGE_VERSION: &str = "0.1.0";

    let name = PackageName::new(PACKAGE_NAME)?;
    let client = create_client(config)?;
    let signing_key = test_signing_key();
    let digest = publish_component(
        &client,
        &name,
        PACKAGE_VERSION,
        "(component)",
        true,
        &signing_key,
    )
    .await?;

    client.upsert([&name]).await?;
    let package = client
        .registry()
        .load_package(&name)
        .await?
        .expect("expected the package to exist");
    package
        .state
        .release(&Version::parse(PACKAGE_VERSION)?)
        .expect("expected the package version to exist");

    // Look up the content URL for the record
    let client = api::Client::new(config.default_url.as_ref().unwrap())?;
    let ContentSourcesResponse { content_sources } = client.content_sources(&digest).await?;
    assert_eq!(content_sources.len(), 1);
    let sources = content_sources
        .get(&digest)
        .expect("expected content source to be provided for the requested digest");
    assert_eq!(sources.len(), 1);

    let expected_url = format!(
        "https://example.com/content/{digest}",
        digest = digest.to_string().replace(':', "-")
    );

    match &sources[0] {
        ContentSource::HttpGet { url, .. } => {
            assert_eq!(url, &expected_url);
        }
    }

    Ok(())
}

async fn test_fetch_package_names(config: &Config) -> Result<()> {
    let name_1 = PackageName::new("test:component")?;
    let log_id_1 = LogId::package_log::<Sha256>(&name_1);

    let url = Url::parse(config.default_url.as_ref().unwrap())?
        .join(paths::fetch_package_names())
        .unwrap();

    let body = FetchPackageNamesRequest {
        packages: Cow::Owned(vec![log_id_1.clone()]),
    };

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .json(&serde_json::to_value(&body).unwrap())
        .send()
        .await?;

    let status = response.status();
    let names_resp = response.json::<FetchPackageNamesResponse>().await?;

    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected response from server: {status}",
    );

    let lookup_name_1 = names_resp.packages.get(&log_id_1);
    assert_eq!(
        lookup_name_1,
        Some(&Some(name_1.clone())),
        "fetch of package name {name_1} mismatched to {lookup_name_1:?}"
    );

    Ok(())
}

async fn test_get_ledger(config: &Config) -> Result<()> {
    let client = api::Client::new(config.default_url.as_ref().unwrap())?;

    let ts_checkpoint = client.latest_checkpoint().await?;
    let checkpoint = &ts_checkpoint.as_ref().checkpoint;

    let url = Url::parse(config.default_url.as_ref().unwrap())?
        .join(paths::ledger_sources())
        .unwrap();

    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    let status = response.status();
    let ledger_sources = response.json::<LedgerSourcesResponse>().await?;

    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected response from server: {status}",
    );

    let hash_algorithm = ledger_sources.hash_algorithm;
    assert_eq!(
        hash_algorithm,
        HashAlgorithm::Sha256,
        "unexpected hash_algorithm: {hash_algorithm}",
    );

    let sources_len = ledger_sources.sources.len();
    assert_eq!(sources_len, 1, "unexpected sources length: {sources_len}",);

    let LedgerSource {
        first_registry_index,
        last_registry_index,
        url,
        content_type,
        ..
    } = ledger_sources.sources.get(0).unwrap();

    assert_eq!(
        content_type,
        &LedgerSourceContentType::Packed,
        "unexpected ledger source content type",
    );
    assert_eq!(
        *first_registry_index, 0,
        "unexpected ledger source first registry index: {first_registry_index}",
    );
    assert_eq!(
        *last_registry_index,
        checkpoint.log_length - 1,
        "unexpected ledger source last registry index: {last_registry_index}",
    );

    let url = Url::parse(config.default_url.as_ref().unwrap())?
        .join(url)
        .unwrap();

    // get ledger source
    let response = client.get(url).send().await?;

    let status = response.status();
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected response from server: {status}",
    );

    let bytes = response.bytes().await?;
    let bytes_len = bytes.len();
    assert_eq!(
        bytes_len,
        checkpoint.log_length * 64,
        "unexpected response body length for ledger source from server: {bytes_len}",
    );

    Ok(())
}
