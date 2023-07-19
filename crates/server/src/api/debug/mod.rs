use axum::{
    debug_handler, extract::State, http::StatusCode, response::IntoResponse, routing::get, Router,
};
use warg_protocol::registry::PackageId;

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
            .with_state(self)
    }
}

#[debug_handler]
async fn list_package_names(
    State(config): State<Config>,
) -> anyhow::Result<Json<Vec<PackageId>>, DebugError> {
    let ids = config.core_service.store().debug_list_package_ids().await?;
    Ok(Json(ids))
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
