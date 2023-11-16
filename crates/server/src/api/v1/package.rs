use super::{Json, Path, RegistryHeader};
use crate::{
    datastore::{DataStoreError, RecordStatus},
    policy::{
        content::{ContentPolicy, ContentPolicyError},
        record::{RecordPolicy, RecordPolicyError},
    },
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
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use warg_api::v1::package::{
    MissingContent, PackageError, PackageRecord, PackageRecordState, PublishRecordRequest,
    UploadEndpoint,
};
use warg_crypto::hash::{AnyHash, Sha256};
use warg_protocol::{
    package,
    registry::{LogId, RecordId},
    ProtoEnvelope, Record as _,
};

#[derive(Clone)]
pub struct Config {
    core_service: CoreService,
    files_dir: PathBuf,
    temp_dir: PathBuf,
    content_policy: Option<Arc<dyn ContentPolicy>>,
    record_policy: Option<Arc<dyn RecordPolicy>>,
}

impl Config {
    pub fn new(
        core_service: CoreService,
        files_dir: PathBuf,
        temp_dir: PathBuf,
        content_policy: Option<Arc<dyn ContentPolicy>>,
        record_policy: Option<Arc<dyn RecordPolicy>>,
    ) -> Self {
        Self {
            core_service,
            files_dir,
            temp_dir,
            content_policy,
            record_policy,
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

    fn content_present(&self, digest: &AnyHash) -> bool {
        self.content_path(digest).is_file()
    }

    fn content_file_name(&self, digest: &AnyHash) -> String {
        digest.to_string().replace(':', "-")
    }

    fn content_path(&self, digest: &AnyHash) -> PathBuf {
        self.files_dir.join(self.content_file_name(digest))
    }

    fn build_missing_content<'a>(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        missing_digests: impl IntoIterator<Item = &'a AnyHash>,
    ) -> HashMap<AnyHash, MissingContent> {
        missing_digests
            .into_iter()
            .map(|digest| {
                let url = format!("v1/package/{log_id}/record/{record_id}/content/{digest}");
                (
                    digest.clone(),
                    MissingContent {
                        upload: vec![UploadEndpoint::Http {
                            method: "POST".to_string(),
                            url,
                            headers: HashMap::new(),
                        }],
                    },
                )
            })
            .collect()
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
            DataStoreError::UnknownKey(_) | DataStoreError::SignatureVerificationFailed(_) => {
                PackageError::Unauthorized(e.to_string())
            }
            DataStoreError::PackageNamespaceNotDefined(id) => PackageError::NamespaceNotDefined(id),
            DataStoreError::PackageNamespaceImported(id) => PackageError::NamespaceImported(id),
            DataStoreError::PackageNameConflict { existing, .. } => {
                PackageError::PackageNameConflict(existing)
            }
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

impl From<ContentPolicyError> for PackageApiError {
    fn from(e: ContentPolicyError) -> Self {
        match e {
            ContentPolicyError::Rejection(message) => Self(PackageError::Rejection(message)),
        }
    }
}

impl From<RecordPolicyError> for PackageApiError {
    fn from(e: RecordPolicyError) -> Self {
        match e {
            RecordPolicyError::Unauthorized(message) => Self(PackageError::Unauthorized(message)),
            RecordPolicyError::Rejection(message) => Self(PackageError::Rejection(message)),
        }
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
    RegistryHeader(_registry_header): RegistryHeader,
    Json(body): Json<PublishRecordRequest<'static>>,
) -> Result<impl IntoResponse, PackageApiError> {
    let expected_log_id = LogId::package_log::<Sha256>(&body.package_id);
    if expected_log_id != log_id {
        return Err(PackageApiError::bad_request(format!(
            "package log identifier `{expected_log_id}` derived from `{id}` does not match provided log identifier `{log_id}`",
            id = body.package_id
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

    // Verify the package name is unique in a case insensitive way and
    // the namespace is defined in the operator log and not imported
    // from another registry.
    config
        .core_service
        .store()
        .verify_can_publish_package(&LogId::operator_log::<Sha256>(), &body.package_id)
        .await?;

    // Preemptively perform the policy check on the record before storing it
    // This is performed here so that we never store an unauthorized record
    if let Some(policy) = &config.record_policy {
        policy.check(&body.package_id, &record)?;
    }

    // Verify the signature on the record itself before storing it
    config
        .core_service
        .store()
        .verify_package_record_signature(&log_id, &record)
        .await?;

    let record_id = RecordId::package_record::<Sha256>(&record);
    let mut missing = record.as_ref().contents();
    missing.retain(|d| !config.content_present(d));

    config
        .core_service
        .store()
        .store_package_record(&log_id, &body.package_id, &record_id, &record, &missing)
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
                record_id,
                state: PackageRecordState::Processing,
            }),
        ));
    }

    let missing_content = config.build_missing_content(&log_id, &record_id, missing);
    Ok((
        StatusCode::ACCEPTED,
        Json(PackageRecord {
            record_id,
            state: PackageRecordState::Sourcing { missing_content },
        }),
    ))
}

#[debug_handler]
async fn get_record(
    State(config): State<Config>,
    Path((log_id, record_id)): Path<(LogId, RecordId)>,
    RegistryHeader(_registry_header): RegistryHeader,
) -> Result<Json<PackageRecord>, PackageApiError> {
    let record = config
        .core_service
        .store()
        .get_package_record(&log_id, &record_id)
        .await?;

    match record.status {
        RecordStatus::MissingContent(missing) => {
            let missing_content = config.build_missing_content(&log_id, &record_id, &missing);
            Ok(Json(PackageRecord {
                record_id,
                state: PackageRecordState::Sourcing { missing_content },
            }))
        }
        // Validated is considered still processing until included in a checkpoint
        RecordStatus::Pending | RecordStatus::Validated => Ok(Json(PackageRecord {
            record_id,
            state: PackageRecordState::Processing,
        })),
        RecordStatus::Rejected(reason) => Ok(Json(PackageRecord {
            record_id,
            state: PackageRecordState::Rejected { reason },
        })),
        RecordStatus::Published => {
            let registry_index = record.registry_index.unwrap();

            Ok(Json(PackageRecord {
                record_id,
                state: PackageRecordState::Published { registry_index },
            }))
        }
    }
}

#[debug_handler]
async fn upload_content(
    State(config): State<Config>,
    Path((log_id, record_id, digest)): Path<(LogId, RecordId, AnyHash)>,
    RegistryHeader(_registry_header): RegistryHeader,
    stream: BodyStream,
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

    let res = process_content(&tmp_path, &digest, stream, config.content_policy.as_deref()).await;

    // If the error was a rejection, transition the record itself to rejected
    if let Err(PackageApiError(PackageError::Rejection(reason))) = &res {
        config
            .core_service
            .store()
            .reject_package_record(
                &log_id,
                &record_id,
                &format!("content with digest `{digest}` was rejected by policy: {reason}"),
            )
            .await?;
    }

    // Only persist the file if the content was successfully processed
    res?;

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
            .submit_package_record(log_id, record_id)
            .await;
    }

    Ok(StatusCode::CREATED)
}

async fn process_content(
    path: &std::path::Path,
    digest: &AnyHash,
    mut stream: BodyStream,
    policy: Option<&dyn ContentPolicy>,
) -> Result<(), PackageApiError> {
    let mut tmp_file = tokio::fs::File::create(&path)
        .await
        .map_err(PackageApiError::internal_error)?;

    let mut hasher = digest.algorithm().hasher();
    let mut policy = policy.map(|p| p.new_stream_policy(digest)).transpose()?;

    while let Some(chunk) = stream
        .next()
        .await
        .transpose()
        .map_err(PackageApiError::internal_error)?
    {
        if let Some(policy) = policy.as_mut() {
            policy.check(&chunk)?;
        }

        hasher.update(&chunk);
        tmp_file
            .write_all(&chunk)
            .await
            .map_err(PackageApiError::internal_error)?;
    }

    let result = hasher.finalize();
    if &result != digest {
        return Err(PackageApiError::bad_request(format!(
            "content digest `{result}` does not match expected digest `{digest}`",
        )));
    }

    if let Some(mut policy) = policy {
        policy.finalize()?;
    }

    Ok(())
}
