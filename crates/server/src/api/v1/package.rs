use super::{Json, Path};
use crate::{
    datastore::{DataStoreError, RecordStatus},
    services::CoreService,
};
use axum::{
    debug_handler,
    extract::{BodyStream, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use futures::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use warg_api::v1::package::{
    ContentSource, PackageError, PackageRecord, PackageRecordState, PublishRecordRequest,
};
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::{
    package,
    registry::{LogId, RecordId},
    ProtoEnvelope, Record as _,
};

#[derive(Clone)]
pub struct Config {
    core_service: Arc<CoreService>,
    base_url: String,
    files_dir: PathBuf,
    temp_dir: PathBuf,
}

impl Config {
    pub fn new(
        core_service: Arc<CoreService>,
        base_url: String,
        files_dir: PathBuf,
        temp_dir: PathBuf,
    ) -> Self {
        Self {
            core_service,
            base_url,
            files_dir,
            temp_dir,
        }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/:log_id/record", post(publish_record))
            .route("/:log_id/record/:record_id", get(get_record))
            .route(
                "/:log_id/record/:record_id/content/:digest",
                post(upload_content),
            )
            .with_state(self)
    }

    fn content_present(&self, digest: &DynHash) -> bool {
        self.content_path(digest).is_file()
    }

    fn content_file_name(&self, digest: &DynHash) -> String {
        digest.to_string().replace(':', "-")
    }

    fn content_path(&self, digest: &DynHash) -> PathBuf {
        self.files_dir.join(self.content_file_name(digest))
    }

    fn content_url(&self, digest: &DynHash) -> String {
        format!(
            "{url}/content/{name}",
            url = self.base_url,
            name = self.content_file_name(digest)
        )
    }
}

struct PackageApiError(PackageError);

impl PackageApiError {
    fn bad_request(message: impl ToString) -> Self {
        Self(PackageError::Message {
            status: StatusCode::BAD_REQUEST.as_u16(),
            message: message.to_string(),
        })
    }

    fn internal_error(e: impl std::fmt::Display) -> Self {
        tracing::error!("unexpected error: {e}");
        Self(PackageError::Message {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            message: "an error occurred while processing the request".into(),
        })
    }

    fn unsupported(message: impl ToString) -> Self {
        Self(PackageError::Message {
            status: StatusCode::NOT_IMPLEMENTED.as_u16(),
            message: message.to_string(),
        })
    }
}

impl From<DataStoreError> for PackageApiError {
    fn from(e: DataStoreError) -> Self {
        Self(match e {
            DataStoreError::PackageValidationFailed(e) => {
                return Self::bad_request(e);
            }
            DataStoreError::LogNotFound(id) => PackageError::LogNotFound(id),
            DataStoreError::RecordNotFound(id) => PackageError::RecordNotFound(id),
            // Other errors are internal server errors
            e => {
                tracing::error!("unexpected data store error: {e}");
                PackageError::Message {
                    status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    message: "an error occurred while processing the request".into(),
                }
            }
        })
    }
}

impl IntoResponse for PackageApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::from_u16(self.0.status()).unwrap(), Json(self.0)).into_response()
    }
}

#[debug_handler]
async fn publish_record(
    State(config): State<Config>,
    Path(log_id): Path<LogId>,
    Json(body): Json<PublishRecordRequest<'static>>,
) -> Result<impl IntoResponse, PackageApiError> {
    let expected_log_id = LogId::package_log::<Sha256>(&body.name);
    if expected_log_id != log_id {
        return Err(PackageApiError::bad_request(format!(
            "package log identifier `{expected_log_id}` derived from name `{name}` does not match provided log identifier `{log_id}`",
            name = body.name
        )));
    }

    let record: ProtoEnvelope<package::PackageRecord> = body
        .record
        .into_owned()
        .try_into()
        .map_err(PackageApiError::bad_request)?;

    // Specifying content sources is not allowed in this implementation
    if !body.content_sources.is_empty() {
        return Err(PackageApiError::unsupported(
            "specifying content sources is not supported",
        ));
    }

    let record_id = RecordId::package_record::<Sha256>(&record);

    let mut missing = record.as_ref().contents();
    missing.retain(|d| !config.content_present(d));

    config
        .core_service
        .store()
        .store_package_record(&log_id, &body.name, &record_id, &record, &missing)
        .await?;

    // If there's no missing content, submit the record for processing now
    if missing.is_empty() {
        config
            .core_service
            .submit_package_record(log_id, record_id.clone())
            .await;

        return Ok((
            StatusCode::ACCEPTED,
            Json(PackageRecord {
                id: record_id,
                state: PackageRecordState::Processing,
            }),
        ));
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(PackageRecord {
            id: record_id,
            state: PackageRecordState::Sourcing {
                missing_content: missing.into_iter().cloned().collect(),
            },
        }),
    ))
}

#[debug_handler]
async fn get_record(
    State(config): State<Config>,
    Path((log_id, record_id)): Path<(LogId, RecordId)>,
) -> Result<Json<PackageRecord>, PackageApiError> {
    let record = config
        .core_service
        .store()
        .get_package_record(&log_id, &record_id)
        .await?;

    match record.status {
        RecordStatus::MissingContent(missing) => Ok(Json(PackageRecord {
            id: record_id,
            state: PackageRecordState::Sourcing {
                missing_content: missing,
            },
        })),
        // Validated is considered still processing until included in a checkpoint
        RecordStatus::Pending | RecordStatus::Validated => Ok(Json(PackageRecord {
            id: record_id,
            state: PackageRecordState::Processing,
        })),
        RecordStatus::Rejected(reason) => Ok(Json(PackageRecord {
            id: record_id,
            state: PackageRecordState::Rejected { reason },
        })),
        RecordStatus::Published => {
            let content_sources = record
                .envelope
                .as_ref()
                .contents()
                .into_iter()
                .map(|d| {
                    (
                        d.clone(),
                        vec![ContentSource::Http {
                            url: config.content_url(d),
                        }],
                    )
                })
                .collect();

            Ok(Json(PackageRecord {
                id: record_id,
                state: PackageRecordState::Published {
                    record: record.envelope.into(),
                    checkpoint: record.checkpoint.unwrap(),
                    content_sources,
                },
            }))
        }
    }
}

#[debug_handler]
async fn upload_content(
    State(config): State<Config>,
    Path((log_id, record_id, digest)): Path<(LogId, RecordId, DynHash)>,
    mut stream: BodyStream,
) -> Result<impl IntoResponse, PackageApiError> {
    match config
        .core_service
        .store()
        .is_content_missing(&log_id, &record_id, &digest)
        .await
    {
        Ok(true) => {}
        Ok(false) => {
            return Err(PackageApiError::bad_request(
                "content digest `{digest}` is not required for package record `{record_id}`",
            ));
        }
        Err(DataStoreError::RecordNotPending(_)) => {
            return Err(PackageApiError(PackageError::RecordNotSourcing))
        }
        Err(e) => return Err(e.into()),
    }

    let tmp_path = NamedTempFile::new_in(&config.temp_dir)
        .map_err(PackageApiError::internal_error)?
        .into_temp_path();

    tracing::debug!(
        "uploading content for record `{record_id}` from `{log_id}` to `{path}`",
        path = tmp_path.display()
    );

    let mut hasher = digest.algorithm().hasher();
    let mut tmp_file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(PackageApiError::internal_error)?;

    while let Some(chunk) = stream
        .next()
        .await
        .transpose()
        .map_err(PackageApiError::internal_error)?
    {
        // TODO: validate each chunk against the content policy

        hasher.update(&chunk);
        tmp_file
            .write_all(&chunk)
            .await
            .map_err(PackageApiError::internal_error)?;
    }

    let result = hasher.finalize();
    if result != digest {
        return Err(PackageApiError::bad_request(format!(
            "content digest `{result}` does not match expected digest `{digest}`",
        )));
    }

    // TODO: if the content is not acceptable (i.e. fails a policy check), we should
    // not persist the file, reject the associated record, and return an error

    tmp_path
        .persist(config.content_path(&digest))
        .map_err(PackageApiError::internal_error)?;

    // If this is the last content needed, submit the record for processing now
    if config
        .core_service
        .store()
        .set_content_present(&log_id, &record_id, &digest)
        .await?
    {
        config
            .core_service
            .submit_package_record(log_id, record_id.clone())
            .await;
    }

    Ok((
        StatusCode::CREATED,
        [(axum::http::header::LOCATION, config.content_url(&digest))],
    ))
}
