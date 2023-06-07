use super::Json;
use crate::datastore::DataStoreError;
use crate::services::CoreService;
use axum::http::StatusCode;
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use warg_api::v1::fetch::{FetchError, FetchLogsRequest, FetchLogsResponse, FetchNamesResponse};
use warg_crypto::hash::Sha256;
use warg_protocol::registry::{LogId, MapCheckpoint};
use warg_protocol::{ProtoEnvelopeBody, SerdeEnvelope};

const DEFAULT_RECORDS_LIMIT: u16 = 100;
const MAX_RECORDS_LIMIT: u16 = 1000;

#[derive(Clone)]
pub struct Config {
    core_service: Arc<CoreService>,
}

impl Config {
    pub fn new(core_service: Arc<CoreService>) -> Self {
        Self { core_service }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/query", get(query))
            .route("/logs", post(fetch_logs))
            .route("/checkpoint", get(fetch_checkpoint))
            .with_state(self)
    }
}

struct FetchApiError(FetchError);

impl FetchApiError {
    fn bad_request(message: impl ToString) -> Self {
        Self(FetchError::Message {
            status: StatusCode::BAD_REQUEST.as_u16(),
            message: message.to_string(),
        })
    }
}

impl From<DataStoreError> for FetchApiError {
    fn from(e: DataStoreError) -> Self {
        Self(match e {
            DataStoreError::CheckpointNotFound(checkpoint) => {
                FetchError::CheckpointNotFound(checkpoint)
            }
            DataStoreError::LogNotFound(log_id) => FetchError::LogNotFound(log_id),
            DataStoreError::RecordNotFound(record_id) => FetchError::RecordNotFound(record_id),
            // Other errors are internal server errors
            e => {
                tracing::error!("unexpected data store error: {e}");
                FetchError::Message {
                    status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    message: "an error occurred while processing the request".into(),
                }
            }
        })
    }
}

impl IntoResponse for FetchApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::from_u16(self.0.status()).unwrap(), Json(self.0)).into_response()
    }
}


#[debug_handler]
async fn query(
    State(config): State<Config>,
) -> Result<Json<FetchNamesResponse>, FetchApiError> {

    let names = config
      .core_service
      .store()
      .get_names()
      .await?;

    Ok(Json(FetchNamesResponse {
      query: names
    }))
}

#[debug_handler]
async fn fetch_logs(
    State(config): State<Config>,
    Json(body): Json<FetchLogsRequest<'static>>,
) -> Result<Json<FetchLogsResponse>, FetchApiError> {
    let limit = body.limit.unwrap_or(DEFAULT_RECORDS_LIMIT);
    if limit == 0 || limit > MAX_RECORDS_LIMIT {
        return Err(FetchApiError::bad_request(format!(
            "invalid records limit value `{limit}`: must be between 1 and {MAX_RECORDS_LIMIT}"
        )));
    }

    let operator: Vec<ProtoEnvelopeBody> = config
        .core_service
        .store()
        .get_operator_records(
            &LogId::operator_log::<Sha256>(),
            &body.root,
            body.operator.as_deref(),
            limit,
        )
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    let mut more = operator.len() == limit as usize;

    let mut map = HashMap::new();
    let packages = body.packages.into_owned();
    for (id, since) in packages {
        let records: Vec<ProtoEnvelopeBody> = config
            .core_service
            .store()
            .get_package_records(&id, &body.root, since.as_ref(), limit)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        more |= records.len() == limit as usize;
        map.insert(id, records);
    }

    Ok(Json(FetchLogsResponse {
        more,
        operator,
        packages: map,
    }))
}

#[debug_handler]
async fn fetch_checkpoint(
    State(config): State<Config>,
) -> Result<Json<SerdeEnvelope<MapCheckpoint>>, FetchApiError> {
    Ok(Json(
        config.core_service.store().get_latest_checkpoint().await?,
    ))
}
