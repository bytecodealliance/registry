use self::support::*;
use anyhow::{Context, Result};
use rand_core::OsRng;
use reqwest::StatusCode;
use std::{
    borrow::Cow,
    fs,
    str::FromStr,
    time::{Duration, SystemTime},
};
use url::Url;
use warg_api::v1::{package::PublishRecordRequest, paths};
use warg_client::{api, ClientError, Config};
use warg_crypto::{hash::Sha256, signing::PrivateKey, Encode, Signable};
use warg_protocol::{
    package::{PackageEntry, PackageRecord, PACKAGE_RECORD_VERSION},
    registry::LogId,
    ProtoEnvelope, ProtoEnvelopeBody,
};
use wit_component::DecodedWasm;

mod support;

mod memory;
#[cfg(feature = "postgres")]
mod postgres;

async fn test_initial_checkpoint(config: &Config) -> Result<()> {
    let client = api::Client::new(config.default_url.as_ref().unwrap())?;

    let checkpoint = client.latest_checkpoint().await?;

    // There should be only a single log entry (the initial operator log entry)
    // As the log leaf differs every time because it contains a timestamp,
    // the log root and map root can't be compared to a baseline value.
    assert_eq!(checkpoint.as_ref().log_length, 1);

    // Ensure the response was signed with the operator key
    let operator_key = PrivateKey::from_str(test_operator_key())?;
    assert_eq!(
        checkpoint.key_id().to_string(),
        operator_key.public_key().fingerprint().to_string()
    );

    // Ensure the signature matches the response
    warg_protocol::registry::MapCheckpoint::verify(
        &operator_key.public_key(),
        &checkpoint.as_ref().encode(),
        checkpoint.signature(),
    )?;

    Ok(())
}

async fn test_component_publishing(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:component";
    const PACKAGE_VERSION: &str = "0.1.0";

    let client = create_client(config)?;
    let signing_key = test_signing_key().parse().unwrap();
    let digest = publish_component(
        &client,
        PACKAGE_NAME,
        PACKAGE_VERSION,
        "(component)",
        true,
        &signing_key,
    )
    .await?;

    // Assert that the package can be downloaded
    client.upsert(&[PACKAGE_NAME]).await?;
    let download = client
        .download(PACKAGE_NAME, &PACKAGE_VERSION.parse()?)
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
    assert!(client
        .download(PACKAGE_NAME, &"0.2.0".parse()?)
        .await?
        .is_none());

    Ok(())
}

async fn test_wit_publishing(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:wit-package";
    const PACKAGE_VERSION: &str = "0.1.0";

    let client = create_client(config)?;
    let signing_key = test_signing_key().parse().unwrap();
    let digest = publish_wit(
        &client,
        PACKAGE_NAME,
        PACKAGE_VERSION,
        &format!("package {PACKAGE_NAME}\nworld foo {{}}"),
        true,
        &signing_key,
    )
    .await?;

    // Assert that the package can be downloaded
    client.upsert(&[PACKAGE_NAME]).await?;
    let download = client
        .download(PACKAGE_NAME, &PACKAGE_VERSION.parse()?)
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
    assert!(client
        .download(PACKAGE_NAME, &"0.2.0".parse()?)
        .await?
        .is_none());

    Ok(())
}

async fn test_wasm_content_policy(config: &Config) -> Result<()> {
    const PACKAGE_NAME: &str = "test:bad-content";
    const PACKAGE_VERSION: &str = "0.1.0";

    // Publish empty content to the server
    // This should be rejected by policy because it is not valid WebAssembly
    let client = create_client(config)?;
    let signing_key = test_signing_key().parse().unwrap();
    match publish(
        &client,
        PACKAGE_NAME,
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
            package,
            record_id,
            reason,
        }) => {
            assert_eq!(package, PACKAGE_NAME);
            assert_eq!(
                reason,
                "content is not valid WebAssembly: unexpected end-of-file (at offset 0x0)"
            );

            // Waiting on the publish should fail with a rejection as well
            match client
                .wait_for_publish(PACKAGE_NAME, &record_id, Duration::from_millis(100))
                .await
                .expect_err("expected wait for publish to fail")
            {
                ClientError::PublishRejected {
                    package,
                    record_id: other,
                    reason,
                } => {
                    assert_eq!(package, PACKAGE_NAME);
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
    let client = create_client(config)?;
    let signing_key = test_signing_key().parse().unwrap();
    publish_component(
        &client,
        PACKAGE_NAME,
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
        publish_component(
            &client,
            PACKAGE_NAME,
            "0.2.0",
            "(component)",
            false,
            &signing_key,
        )
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
    let client = create_client(config)?;
    let signing_key = test_signing_key().parse().unwrap();
    publish_component(
        &client,
        PACKAGE_NAME,
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
        publish_component(
            &client,
            PACKAGE_NAME,
            "0.2.0",
            "(component)",
            false,
            &signing_key,
        )
        .await
        .expect_err("expected publish to fail")
    );

    assert!(
        message.contains("unknown key id"),
        "unexpected error message: {message}"
    );

    Ok(())
}

async fn test_invalid_signature(config: &Config) -> Result<()> {
    // Use a reqwest client directly here as we're going to be sending an invalid signature
    let log_id = LogId::package_log::<Sha256>("test:invalid-signature");
    let url = Url::parse(config.default_url.as_ref().unwrap())?
        .join(&paths::publish_package_record(&log_id))
        .unwrap();

    let signing_key = test_signing_key().parse().unwrap();
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
        name: "test:invalid-signature".into(),
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
        body.contains("record signature verification failed"),
        "unexpected response body: {body}"
    );

    Ok(())
}
