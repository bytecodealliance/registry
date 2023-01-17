use anyhow::Result;
use axum::{debug_handler, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::AnyError;

struct ProofConfig {}

impl ProofConfig {
    pub fn build_router(self) -> Result<Router> {
        let router = Router::new()
            .route("/consistency", post(consistency::prove))
            .route("/inclusion", post(inclusion::prove));

        Ok(router)
    }
}

mod consistency {
    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody {
        old_root: String,
        new_root: String,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct ResponseBody {
        proof: Vec<u8>,
    }

    #[debug_handler]
    pub(crate) async fn prove(
        Json(body): Json<RequestBody>,
    ) -> Result<impl IntoResponse, AnyError> {
        let response = ResponseBody { proof: todo!() };

        Ok((StatusCode::OK, Json(response)))
    }
}

mod inclusion {
    use super::*;

    /// ```json
    /// {
    ///     "root": "sha256:fdslkgfdshfds",
    ///     "logs": [
    ///         {
    ///             "name": "foobar",
    ///             "head": "sha256:fdslkgfdshfds"
    ///         }
    ///     ]
    /// }
    /// ```
    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody {
        root: String,
        logs: Vec<LogHead>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct LogHead {
        name: String,
        head: String,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct ResponseBody {
        proof: Vec<u8>,
    }

    #[debug_handler]
    pub(crate) async fn prove(
        Json(body): Json<RequestBody>,
    ) -> Result<impl IntoResponse, AnyError> {
        let response = ResponseBody { proof: todo!() };

        Ok((StatusCode::OK, Json(response)))
    }
}
