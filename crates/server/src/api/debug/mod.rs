use std::time::SystemTime;

use anyhow::Context;
use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Serialize;
use warg_crypto::{
    hash::{AnyHash, Sha256},
    signing::KeyID,
};
use warg_protocol::{
    package::{LogState, Permission, Release},
    registry::{LogId, PackageId, RecordId},
    Version,
};

use crate::{api::v1::Json, services::CoreService};

#[derive(Clone)]
pub struct Config {
    core_service: CoreService,
}

impl Config {
    pub fn new(core_service: CoreService) -> Self {
        Self { core_service }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/packages", get(list_package_names))
            .route("/package/:package_id", get(get_package_info))
            .with_state(self)
    }
}

#[debug_handler]
async fn list_package_names(
    State(config): State<Config>,
) -> Result<Json<Vec<PackageId>>, DebugError> {
    let ids = config.core_service.store().debug_list_package_ids().await?;
    Ok(Json(ids))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageInfo {
    package_id: PackageId,
    log_id: LogId,
    records: Vec<RecordInfo>,
    releases: Vec<Release>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordInfo {
    record_id: RecordId,
    timestamp: u64,
    entries: Vec<EntryInfo>,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct EntryInfo {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key_id: Option<KeyID>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    permissions: Vec<Permission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<Version>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<AnyHash>,
}

#[debug_handler]
async fn get_package_info(
    State(config): State<Config>,
    Path(package_id): Path<PackageId>,
) -> Result<Json<PackageInfo>, DebugError> {
    let store = config.core_service.store();

    let checkpoint = store
        .get_latest_checkpoint()
        .await
        .context("get_latest_checkpoint")?;
    let checkpoint_log_length = checkpoint.as_ref().checkpoint.log_length;

    let log_id = LogId::package_log::<Sha256>(&package_id);
    let records = store
        .get_package_records(&log_id, checkpoint_log_length, None, u16::MAX)
        .await
        .context("get_package_records")?;

    let mut package_state = LogState::new();

    let records = records
        .into_iter()
        .map(|record| {
            package_state
                .validate(&record.envelope)
                .context("validate")?;
            let record_id = RecordId::package_record::<Sha256>(&record.envelope.content_bytes());
            let timestamp = record
                .envelope
                .as_ref()
                .timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .context("duration_since")?
                .as_secs();
            let entries = record
                .envelope
                .as_ref()
                .entries
                .iter()
                .map(|entry| {
                    use warg_protocol::package::PackageEntry::*;
                    match entry {
                        Init { key, .. } => EntryInfo {
                            kind: "init",
                            key: Some(key.to_string()),
                            ..Default::default()
                        },
                        GrantFlat { key, permissions } => EntryInfo {
                            kind: "grant",
                            key: Some(key.to_string()),
                            permissions: permissions.clone(),
                            ..Default::default()
                        },
                        RevokeFlat {
                            key_id,
                            permissions,
                        } => EntryInfo {
                            kind: "revoke",
                            key_id: Some(key_id.clone()),
                            permissions: permissions.clone(),
                            ..Default::default()
                        },
                        Release { version, content } => EntryInfo {
                            kind: "release",
                            version: Some(version.clone()),
                            content: Some(content.clone()),
                            ..Default::default()
                        },
                        Yank { version } => EntryInfo {
                            kind: "yank",
                            version: Some(version.clone()),
                            ..Default::default()
                        },
                        _ => EntryInfo {
                            kind: "UNKNOWN",
                            ..Default::default()
                        },
                    }
                })
                .collect();
            Ok(RecordInfo {
                record_id,
                timestamp,
                entries,
            })
        })
        .collect::<Result<_, DebugError>>()?;

    let releases = package_state.releases().cloned().collect();

    Ok(Json(PackageInfo {
        package_id,
        log_id,
        records,
        releases,
    }))
}

struct DebugError(String);

impl From<anyhow::Error> for DebugError {
    fn from(err: anyhow::Error) -> Self {
        Self(format!("{err:#?}"))
    }
}

impl IntoResponse for DebugError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}
