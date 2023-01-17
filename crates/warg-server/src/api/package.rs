// https://hackmd.io/1-PXR239TyuEj_d5Eq06Pw?view

// Publish Package Record
// POST /package/<package-id>

// Package Summary
// GET /package/<package-id>

// Package Record
// GET /package/<package-id>/<record-id>

use anyhow::Result;
use axum::{
    debug_handler, extract::Path, http::StatusCode, response::IntoResponse, routing::post, Json,
    Router,
};
use serde::{Deserialize, Serialize};

use warg_protocol::Envelope;

use crate::services;
use crate::AnyError;

pub struct PackageConfig {}

impl PackageConfig {
    pub fn build_router(self) -> Result<Router> {
        let router = Router::new().route("/package/:package_id", post(publish::publish));

        Ok(router)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ContentSource {
    HttpAnonymous { content_digest: String, url: String },
}

mod publish {
    use warg_crypto::hash::Sha256;
    use warg_protocol::registry::{LogId, RecordId};

    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody {
        record: Vec<u8>,
        content_sources: Vec<ContentSource>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) enum ResponseBody {
        Published { record_url: String },
        Rejected { reason: String },
        Processing {},
    }

    #[debug_handler]
    pub(crate) async fn publish(
        Path(package_id): Path<String>,
        Json(body): Json<RequestBody>,
    ) -> Result<impl IntoResponse, AnyError> {
        let record = Envelope::from_bytes(body.record)?;

        let info = services::PublishInfo {
            package_id: LogId::package_log::<Sha256>(package_id),
            record_id: RecordId::package_record::<Sha256>(&record),
            record,
            content_sources: body.content_sources,
        };

        Ok((StatusCode::OK, Json(ResponseBody::Processing {})))
    }
}
