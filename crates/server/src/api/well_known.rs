use std::{fs, net::SocketAddr};

use axum::{
    debug_handler, extract::State, http::StatusCode, response::IntoResponse, routing::get, Router,
};
use warg_api::well_known::{DomainOption, WargRegistry, WellKnown, WellKnownError};

use super::v1::{Json, RegistryHeader};

// use super::{Json, RegistryHeader};

#[derive(Clone)]
pub struct Config {
    well_known: WellKnown,
}

impl Config {
    pub fn new(addr: SocketAddr) -> Self {
        let mut well_known: WellKnown =
            serde_json::from_slice(&fs::read("./.well-known.json").unwrap()).unwrap();
        well_known.warg = Some(WargRegistry {
            domain_option: DomainOption::Domain(match addr {
                SocketAddr::V4(v4) => format!("http://{v4}").to_string(),
                SocketAddr::V6(v6) => format!("http://{v6}").to_string(),
            }),
        });
        Self { well_known }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/", get(fetch_well_known))
            .with_state(self)
    }
}

struct WellKnownApiError(WellKnownError);

#[debug_handler]
async fn fetch_well_known(
    State(config): State<Config>,
    RegistryHeader(_registry_header): RegistryHeader,
) -> Result<Json<WellKnown>, WellKnownApiError> {
    Ok(Json(config.well_known))
}

impl IntoResponse for WellKnownApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::from_u16(self.0.status()).unwrap(), Json(self.0))
            .0
            .into_response()
    }
}
