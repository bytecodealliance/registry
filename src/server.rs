use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    body::Bytes,
    extract::Path,
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::{
    digest::TypedDigest,
    maintainer::MaintainerKey,
    release::{
        ContentSource, PublishRelease, Release, ReleaseManifest, UnpublishedRelease,
        UnpublishedReleaseStatus, RELEASE_PAYLOAD_TYPE,
    },
    Error,
};

#[derive(Default)]
pub struct Server {
    data: RwLock<Data>,
}

#[derive(Default)]
struct Data {
    releases: HashMap<String, Release>,
    unpublished_releases: HashMap<String, UnpublishedRelease>,
    release_content: HashMap<TypedDigest, Bytes>,
    maintainer_keys: HashMap<String, MaintainerKey>,
}

type ServerExtension = Extension<Arc<Server>>;

#[axum_macros::debug_handler]
async fn create_unpublished_release(
    unparsed_manifest: String,
    Extension(server): ServerExtension,
) -> Result<impl IntoResponse, ServerError> {
    // Parse manifest
    let manifest: ReleaseManifest =
        serde_json::from_str(&unparsed_manifest).map_err(Error::from)?;

    // Prepare unpublished release
    let upload_url = Some(format!(
        "/prototype/release-content/{}",
        manifest.content_digest
    ));
    let unpublished = UnpublishedRelease {
        release: unparsed_manifest,
        status: UnpublishedReleaseStatus::Pending,
        error: None,
        upload_url,
    };

    // Prepare Location header
    let mut headers = HeaderMap::new();
    let location = format!("{}/unpublished", manifest.resource_path());
    headers.insert("Location", location.parse().unwrap());

    // Update "database"
    let mut data = server.data.write().await;
    let key = manifest.resource_path();
    if data.releases.contains_key(&key) || data.unpublished_releases.contains_key(&key) {
        return Err(Error::ReleaseAlreadyExists.into());
    }
    data.unpublished_releases.insert(key, unpublished.clone());

    Ok((StatusCode::CREATED, headers, Json(unpublished)))
}

#[axum_macros::debug_handler]
async fn get_unpublished_release(
    uri: Uri,
    Extension(server): ServerExtension,
) -> impl IntoResponse {
    let key = uri.path().strip_suffix("/unpublished").unwrap();
    match server.data.read().await.unpublished_releases.get(key) {
        Some(release) => Ok(Json(release).into_response()),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn publish_release(
    uri: Uri,
    Json(publish): Json<PublishRelease>,
    Extension(server): ServerExtension,
) -> impl IntoResponse {
    let Data {
        ref mut releases,
        ref mut unpublished_releases,
        ref mut release_content,
        ref mut maintainer_keys,
    } = *server.data.write().await;

    // Look up unpublished release
    let key = uri.path().strip_suffix("/publish").unwrap();
    let unpublished = match unpublished_releases.get_mut(key) {
        Some(unpublished) => unpublished,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                "Unpublished release not found".to_string(),
            ))
        }
    };

    // Check that content has been uploaded
    if let Some(ref upload_url) = unpublished.upload_url {
        let (_, digest_str) = upload_url.rsplit_once('/').unwrap();
        let digest = digest_str.parse().expect("bad digest in upload_url");
        if !release_content.contains_key(&digest) {
            return Err((
                StatusCode::BAD_REQUEST,
                "Cannot publish; no uploaded content".to_string(),
            ));
        }
    }

    // Verify signature
    let key_id = publish.signature.key_id.as_deref().ok_or((
        StatusCode::BAD_REQUEST,
        "Missing signature key ID".to_string(),
    ))?;
    let maintainer_key = maintainer_keys.get(key_id).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("Unknown key ID {:?}", key_id),
        )
    })?;
    maintainer_key
        .public_key
        .verify_payload(
            RELEASE_PAYLOAD_TYPE,
            unpublished.release.as_bytes(),
            &publish.signature,
        )
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

    // Create release & update "database"
    let unpublished = unpublished_releases.remove(key).unwrap();
    let release = Release {
        release: unpublished.release,
        release_signature: publish.signature,
        content_sources: unpublished
            .upload_url
            .into_iter()
            .map(|url| ContentSource { url })
            .collect(),
    };
    if let Some(existing) = releases.insert(key.to_string(), release) {
        tracing::warn!("Publish somehow overwrote existing release: {:?}", existing);
    }
    tracing::debug!("Published {}", key);

    let release = releases.get(key).unwrap();
    Ok(Json(release).into_response())
}

async fn get_release(uri: Uri, Extension(server): ServerExtension) -> impl IntoResponse {
    let key = uri.path();
    match server.data.read().await.releases.get(key) {
        Some(release) => Ok(Json(release).into_response()),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// Prototype handlers

async fn register_maintainer_key(
    Json(mut maintainer_key): Json<MaintainerKey>,
    Extension(server): ServerExtension,
) -> impl IntoResponse {
    if !maintainer_key.id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot set 'id' on register".to_string(),
        ));
    }

    // Derive key ID from public key fingerprint
    let id = base64::encode(&maintainer_key.public_key.fingerprint()[..16]);
    maintainer_key.id = id.clone();

    // Update "database"
    let maintainer_keys = &mut server.data.write().await.maintainer_keys;
    if maintainer_keys.contains_key(&id) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Public key already registered".to_string(),
        ));
    }
    maintainer_keys.insert(id, maintainer_key.clone());

    Ok(Json(maintainer_key))
}

async fn get_maintainer_public_keys(Extension(server): ServerExtension) -> impl IntoResponse {
    let data = &server.data.read().await;
    Json(
        data.maintainer_keys
            .iter()
            .map(|(id, key)| (id, &key.public_key))
            .collect::<HashMap<_, _>>(),
    )
    .into_response()
}

async fn get_release_content(
    Path(digest): Path<String>,
    Extension(server): ServerExtension,
) -> impl IntoResponse {
    let digest = digest
        .parse()
        .map_err(|err: Error| (StatusCode::BAD_REQUEST, err.to_string()))?;
    match server.data.read().await.release_content.get(&digest) {
        Some(content) => Ok(content.clone()),
        None => Err((StatusCode::NOT_FOUND, "Content not found".to_string())),
    }
}

async fn upload_release_content(
    digest: Path<String>,
    content: Bytes,
    Extension(server): ServerExtension,
) -> impl IntoResponse {
    let digest: TypedDigest = digest
        .parse()
        .map_err(|err| (StatusCode::BAD_REQUEST, format!("Invalid digest: {}", err)))?;

    // Check if content already upload
    if server
        .data
        .read()
        .await
        .release_content
        .contains_key(&digest)
    {
        return Ok((StatusCode::OK, "Content already exists"));
    }

    // Verify digest
    match &digest {
        TypedDigest::Dummy(_) => {
            return Ok((StatusCode::OK, "Upload for 'dummy' digest does nothing"))
        }
        TypedDigest::Sha256(sha256_digest) => {
            let actual_digest = Sha256::digest(&content);
            if &actual_digest[..] != sha256_digest.as_ref() {
                tracing::warn!(
                    "Content digest mismatch; got {:?} want {:?}",
                    &actual_digest,
                    &digest,
                );
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Content doesn't match digest".to_string(),
                ));
            }
        }
    }

    // Update "database"
    server
        .data
        .write()
        .await
        .release_content
        .insert(digest, content);

    Ok((StatusCode::CREATED, "Upload complete"))
}

impl Server {
    pub async fn run(self, bind_addr: &SocketAddr) {
        let server = Arc::new(self);

        let app = Router::new()
            .route("/releases/", post(create_unpublished_release))
            .route(
                "/:entity_collection/:entity_name/v:version/unpublished",
                get(get_unpublished_release),
            )
            .route(
                "/:entity_collection/:entity_name/v:version/publish",
                post(publish_release),
            )
            .route(
                "/:entity_collection/:entity_name/v:version",
                get(get_release),
            )
            // Prototype routes to enable testing
            .route(
                "/prototype/register-maintainer-key",
                post(register_maintainer_key),
            )
            .route(
                "/prototype/maintainer-public-keys",
                get(get_maintainer_public_keys),
            )
            .route(
                "/prototype/release-content/:content_digest",
                post(upload_release_content),
            )
            .route(
                "/prototype/release-content/:content_digest",
                get(get_release_content),
            )
            .layer(Extension(server));

        axum::Server::bind(bind_addr)
            .serve(app.into_make_service())
            .await
            .unwrap()
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

#[derive(Debug, thiserror::Error)]
enum ServerError {
    #[error("{0}")]
    RegistryError(#[from] Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}
