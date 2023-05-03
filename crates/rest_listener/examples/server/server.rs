//! Main library entry point for openapi_client implementation.

#![allow(unused_imports)]

use async_trait::async_trait;
use futures::{future, Stream, StreamExt, TryFutureExt, TryStreamExt};
use hyper::server::conn::Http;
use hyper::service::Service;
use log::info;
use std::future::Future;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use swagger::{Has, XSpanIdString};
use swagger::auth::MakeAllowAllAuthenticator;
use swagger::EmptyContext;
use tokio::net::TcpListener;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "ios")))]
use openssl::ssl::{Ssl, SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};

use openapi_client::models;

/// Builds an SSL implementation for Simple HTTPS from some hard-coded file names
pub async fn create(addr: &str, https: bool) {
    let addr = addr.parse().expect("Failed to parse bind address");

    let server = Server::new();

    let service = MakeService::new(server);

    let service = MakeAllowAllAuthenticator::new(service, "cosmo");

    #[allow(unused_mut)]
    let mut service =
        openapi_client::server::context::MakeAddContext::<_, EmptyContext>::new(
            service
        );

    if https {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "ios"))]
        {
            unimplemented!("SSL is not implemented for the examples on MacOS, Windows or iOS");
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "ios")))]
        {
            let mut ssl = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).expect("Failed to create SSL Acceptor");

            // Server authentication
            ssl.set_private_key_file("examples/server-key.pem", SslFiletype::PEM).expect("Failed to set private key");
            ssl.set_certificate_chain_file("examples/server-chain.pem").expect("Failed to set certificate chain");
            ssl.check_private_key().expect("Failed to check private key");

            let tls_acceptor = ssl.build();
            let tcp_listener = TcpListener::bind(&addr).await.unwrap();

            loop {
                if let Ok((tcp, _)) = tcp_listener.accept().await {
                    let ssl = Ssl::new(tls_acceptor.context()).unwrap();
                    let addr = tcp.peer_addr().expect("Unable to get remote address");
                    let service = service.call(addr);

                    tokio::spawn(async move {
                        let tls = tokio_openssl::SslStream::new(ssl, tcp).map_err(|_| ())?;
                        let service = service.await.map_err(|_| ())?;

                        Http::new()
                            .serve_connection(tls, service)
                            .await
                            .map_err(|_| ())
                    });
                }
            }
        }
    } else {
        // Using HTTP
        hyper::server::Server::bind(&addr).serve(service).await.unwrap()
    }
}

#[derive(Copy, Clone)]
pub struct Server<C> {
    marker: PhantomData<C>,
}

impl<C> Server<C> {
    pub fn new() -> Self {
        Server{marker: PhantomData}
    }
}


use openapi_client::{
    Api,
    WargFetchLogsResponse,
    WargGetPackageResponse,
    WargGetPackageRecordResponse,
    WargPublishPackageResponse,
    WargFetchCheckpointResponse,
    WargProveConsistencyResponse,
    WargProveInclusionResponse,
};
use openapi_client::server::MakeService;
use std::error::Error;
use swagger::ApiError;

#[async_trait]
impl<C> Api<C> for Server<C> where C: Has<XSpanIdString> + Send + Sync
{
    /// Fetches logs for a requested package.
    async fn warg_fetch_logs(
        &self,
        root_period_algo: Option<String>,
        root_period_bytes: Option<swagger::ByteArray>,
        operator_period_algo: Option<String>,
        operator_period_bytes: Option<swagger::ByteArray>,
        context: &C) -> Result<WargFetchLogsResponse, ApiError>
    {
        let context = context.clone();
        info!("warg_fetch_logs({:?}, {:?}, {:?}, {:?}) - X-Span-ID: {:?}", root_period_algo, root_period_bytes, operator_period_algo, operator_period_bytes, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Used for polling while package is being in the processed of publishing.
    async fn warg_get_package(
        &self,
        package_id: String,
        context: &C) -> Result<WargGetPackageResponse, ApiError>
    {
        let context = context.clone();
        info!("warg_get_package(\"{}\") - X-Span-ID: {:?}", package_id, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Get a specific record within a package.
    async fn warg_get_package_record(
        &self,
        package_id: String,
        record_id: String,
        context: &C) -> Result<WargGetPackageRecordResponse, ApiError>
    {
        let context = context.clone();
        info!("warg_get_package_record(\"{}\", \"{}\") - X-Span-ID: {:?}", package_id, record_id, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Request that a new package be published.
    async fn warg_publish_package(
        &self,
        name: Option<String>,
        record_period_contents: Option<swagger::ByteArray>,
        record_period_key_id: Option<String>,
        record_period_signature: Option<String>,
        context: &C) -> Result<WargPublishPackageResponse, ApiError>
    {
        let context = context.clone();
        info!("warg_publish_package({:?}, {:?}, {:?}, {:?}) - X-Span-ID: {:?}", name, record_period_contents, record_period_key_id, record_period_signature, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Fetches logs for a root.
    async fn warg_fetch_checkpoint(
        &self,
        context: &C) -> Result<WargFetchCheckpointResponse, ApiError>
    {
        let context = context.clone();
        info!("warg_fetch_checkpoint() - X-Span-ID: {:?}", context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Proves consistency between an old root and a new one.
    async fn warg_prove_consistency(
        &self,
        old_root_period_algo: Option<String>,
        old_root_period_bytes: Option<swagger::ByteArray>,
        new_root_period_algo: Option<String>,
        new_root_period_bytes: Option<swagger::ByteArray>,
        context: &C) -> Result<WargProveConsistencyResponse, ApiError>
    {
        let context = context.clone();
        info!("warg_prove_consistency({:?}, {:?}, {:?}, {:?}) - X-Span-ID: {:?}", old_root_period_algo, old_root_period_bytes, new_root_period_algo, new_root_period_bytes, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Proves inclusion between a log and a map.
    async fn warg_prove_inclusion(
        &self,
        checkpoint_period_log_root_period_algo: Option<String>,
        checkpoint_period_log_root_period_bytes: Option<swagger::ByteArray>,
        checkpoint_period_log_length: Option<i64>,
        checkpoint_period_map_root_period_algo: Option<String>,
        checkpoint_period_map_root_period_bytes: Option<swagger::ByteArray>,
        context: &C) -> Result<WargProveInclusionResponse, ApiError>
    {
        let context = context.clone();
        info!("warg_prove_inclusion({:?}, {:?}, {:?}, {:?}, {:?}) - X-Span-ID: {:?}", checkpoint_period_log_root_period_algo, checkpoint_period_log_root_period_bytes, checkpoint_period_log_length, checkpoint_period_map_root_period_algo, checkpoint_period_map_root_period_bytes, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

}
