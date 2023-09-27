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
use warg_api::v1::fetch::{FetchError, FetchLogsRequest, FetchLogsResponse, PublishedRecord};
use warg_crypto::hash::{AnyHash, Sha256};
use warg_protocol::registry::{LogId, RecordId, TimestampedCheckpoint};
use warg_protocol::SerdeEnvelope;

const DEFAULT_RECORDS_LIMIT: u16 = 100;
const MAX_RECORDS_LIMIT: u16 = 1000;

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
            DataStoreError::RecordNotFound(record_id) => {
                FetchError::FetchTokenNotFound(record_id.to_string())
            }
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

    let operator_fetch_token: Option<RecordId> = match body.operator {
        Some(s) => Some(
            s.parse::<AnyHash>()
                .map_err(|_| FetchApiError(FetchError::FetchTokenNotFound(s.into_owned())))?
                .into(),
        ),
        None => None,
    };
    let operator: Vec<PublishedRecord> = config
        .core_service
        .store()
        .get_operator_records(
            &LogId::operator_log::<Sha256>(),
            body.log_length,
            operator_fetch_token.as_ref(),
            limit,
        )
        .await?
        .into_iter()
        .map(|envelope| {
            // use the record ID as the fetch token
            let fetch_token = RecordId::operator_record::<Sha256>(&envelope.envelope).to_string();
            PublishedRecord {
                envelope: envelope.into(),
                fetch_token,
            }
        })
        .collect();

    let mut more = operator.len() == limit as usize;

    let mut map = HashMap::new();
    let packages = body.packages.into_owned();
    for (id, fetch_token) in packages {
        let since: Option<RecordId> = match fetch_token {
            Some(s) => Some(
                s.parse::<AnyHash>()
                    .map_err(|_| FetchApiError(FetchError::FetchTokenNotFound(s)))?
                    .into(),
            ),
            None => None,
        };
        let records: Vec<PublishedRecord> = config
            .core_service
            .store()
            .get_package_records(&id, body.log_length, since.as_ref(), limit)
            .await?
            .into_iter()
            .map(|envelope| {
                // use the record ID as the fetch token
                let fetch_token =
                    RecordId::package_record::<Sha256>(&envelope.envelope).to_string();
                PublishedRecord {
                    envelope: envelope.into(),
                    fetch_token,
                }
            })
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
) -> Result<Json<SerdeEnvelope<TimestampedCheckpoint>>, FetchApiError> {
    Ok(Json(
        config.core_service.store().get_latest_checkpoint().await?,
    ))
}
