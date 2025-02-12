//! An authentication layer to secure endpoints with a bearer token.
use axum::{
    http::{Request, StatusCode},
    response::Response,
};
use futures::future::BoxFuture;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tower::Layer;

#[derive(Clone)]
pub struct AuthLayer {
    secret: Arc<String>,
}

impl AuthLayer {
    pub fn new<S: Into<String>>(secret: S) -> Self {
        AuthLayer {
            secret: Arc::new(secret.into()),
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,

            secret: self.secret.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,

    secret: Arc<String>,
}

impl<S, B> tower::Service<Request<B>> for AuthService<S>
where
    S: tower::Service<Request<B>, Response = Response> + Send + 'static,

    S::Future: Send + 'static,

    B: Send + 'static,
{
    type Response = Response;

    type Error = S::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let secret = self.secret.clone();
        let expected = format!("Bearer {}", secret);
        let header_value_opt = req
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok());
        if let Some(header_value) = header_value_opt {
            if header_value == expected {
                // The header is correct, forward the request.
                return Box::pin(self.inner.call(req));
            }
        }
        // The header is incorrect, return an Unauthorized response.
        Box::pin(async move {
            Ok(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body("Unauthorized".into())
                .unwrap())
        })
    }
}
