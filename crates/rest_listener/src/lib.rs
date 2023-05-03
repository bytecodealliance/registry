#![allow(missing_docs, trivial_casts, unused_variables, unused_mut, unused_imports, unused_extern_crates, non_camel_case_types)]
#![allow(unused_imports, unused_attributes)]
#![allow(clippy::derive_partial_eq_without_eq, clippy::disallowed_names)]

use async_trait::async_trait;
use futures::Stream;
use std::error::Error;
use std::task::{Poll, Context};
use swagger::{ApiError, ContextWrapper};
use serde::{Serialize, Deserialize};

type ServiceError = Box<dyn Error + Send + Sync + 'static>;

pub const BASE_PATH: &str = "";
pub const API_VERSION: &str = "1.0";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum WargFetchLogsResponse {
    /// A successful response.
    ASuccessfulResponse
    (models::V1FetchLogsResponse)
    ,
    /// An unexpected error response.
    AnUnexpectedErrorResponse
    (models::RpcStatus)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum WargGetPackageResponse {
    /// A successful response.
    ASuccessfulResponse
    (models::V1GetPackageResponse)
    ,
    /// An unexpected error response.
    AnUnexpectedErrorResponse
    (models::RpcStatus)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum WargGetPackageRecordResponse {
    /// A successful response.
    ASuccessfulResponse
    (models::V1Record)
    ,
    /// An unexpected error response.
    AnUnexpectedErrorResponse
    (models::RpcStatus)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum WargPublishPackageResponse {
    /// A successful response.
    ASuccessfulResponse
    (models::V1PublishPackageResponse)
    ,
    /// An unexpected error response.
    AnUnexpectedErrorResponse
    (models::RpcStatus)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum WargFetchCheckpointResponse {
    /// A successful response.
    ASuccessfulResponse
    (models::V1FetchCheckpointResponse)
    ,
    /// An unexpected error response.
    AnUnexpectedErrorResponse
    (models::RpcStatus)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum WargProveConsistencyResponse {
    /// A successful response.
    ASuccessfulResponse
    (models::V1ProveConsistencyResponse)
    ,
    /// An unexpected error response.
    AnUnexpectedErrorResponse
    (models::RpcStatus)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum WargProveInclusionResponse {
    /// A successful response.
    ASuccessfulResponse
    (models::V1ProveInclusionResponse)
    ,
    /// An unexpected error response.
    AnUnexpectedErrorResponse
    (models::RpcStatus)
}

/// API
#[async_trait]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub trait Api<C: Send + Sync> {
    fn poll_ready(&self, _cx: &mut Context) -> Poll<Result<(), Box<dyn Error + Send + Sync + 'static>>> {
        Poll::Ready(Ok(()))
    }

    /// Fetches logs for a requested package.
    async fn warg_fetch_logs(
        &self,
        root_period_algo: Option<String>,
        root_period_bytes: Option<swagger::ByteArray>,
        operator_period_algo: Option<String>,
        operator_period_bytes: Option<swagger::ByteArray>,
        context: &C) -> Result<WargFetchLogsResponse, ApiError>;

    /// Used for polling while package is being in the processed of publishing.
    async fn warg_get_package(
        &self,
        package_id: String,
        context: &C) -> Result<WargGetPackageResponse, ApiError>;

    /// Get a specific record within a package.
    async fn warg_get_package_record(
        &self,
        package_id: String,
        record_id: String,
        context: &C) -> Result<WargGetPackageRecordResponse, ApiError>;

    /// Request that a new package be published.
    async fn warg_publish_package(
        &self,
        name: Option<String>,
        record_period_contents: Option<swagger::ByteArray>,
        record_period_key_id: Option<String>,
        record_period_signature: Option<String>,
        context: &C) -> Result<WargPublishPackageResponse, ApiError>;

    /// Fetches logs for a root.
    async fn warg_fetch_checkpoint(
        &self,
        context: &C) -> Result<WargFetchCheckpointResponse, ApiError>;

    /// Proves consistency between an old root and a new one.
    async fn warg_prove_consistency(
        &self,
        old_root_period_algo: Option<String>,
        old_root_period_bytes: Option<swagger::ByteArray>,
        new_root_period_algo: Option<String>,
        new_root_period_bytes: Option<swagger::ByteArray>,
        context: &C) -> Result<WargProveConsistencyResponse, ApiError>;

    /// Proves inclusion between a log and a map.
    async fn warg_prove_inclusion(
        &self,
        checkpoint_period_log_root_period_algo: Option<String>,
        checkpoint_period_log_root_period_bytes: Option<swagger::ByteArray>,
        checkpoint_period_log_length: Option<i64>,
        checkpoint_period_map_root_period_algo: Option<String>,
        checkpoint_period_map_root_period_bytes: Option<swagger::ByteArray>,
        context: &C) -> Result<WargProveInclusionResponse, ApiError>;

}

/// API where `Context` isn't passed on every API call
#[async_trait]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub trait ApiNoContext<C: Send + Sync> {

    fn poll_ready(&self, _cx: &mut Context) -> Poll<Result<(), Box<dyn Error + Send + Sync + 'static>>>;

    fn context(&self) -> &C;

    /// Fetches logs for a requested package.
    async fn warg_fetch_logs(
        &self,
        root_period_algo: Option<String>,
        root_period_bytes: Option<swagger::ByteArray>,
        operator_period_algo: Option<String>,
        operator_period_bytes: Option<swagger::ByteArray>,
        ) -> Result<WargFetchLogsResponse, ApiError>;

    /// Used for polling while package is being in the processed of publishing.
    async fn warg_get_package(
        &self,
        package_id: String,
        ) -> Result<WargGetPackageResponse, ApiError>;

    /// Get a specific record within a package.
    async fn warg_get_package_record(
        &self,
        package_id: String,
        record_id: String,
        ) -> Result<WargGetPackageRecordResponse, ApiError>;

    /// Request that a new package be published.
    async fn warg_publish_package(
        &self,
        name: Option<String>,
        record_period_contents: Option<swagger::ByteArray>,
        record_period_key_id: Option<String>,
        record_period_signature: Option<String>,
        ) -> Result<WargPublishPackageResponse, ApiError>;

    /// Fetches logs for a root.
    async fn warg_fetch_checkpoint(
        &self,
        ) -> Result<WargFetchCheckpointResponse, ApiError>;

    /// Proves consistency between an old root and a new one.
    async fn warg_prove_consistency(
        &self,
        old_root_period_algo: Option<String>,
        old_root_period_bytes: Option<swagger::ByteArray>,
        new_root_period_algo: Option<String>,
        new_root_period_bytes: Option<swagger::ByteArray>,
        ) -> Result<WargProveConsistencyResponse, ApiError>;

    /// Proves inclusion between a log and a map.
    async fn warg_prove_inclusion(
        &self,
        checkpoint_period_log_root_period_algo: Option<String>,
        checkpoint_period_log_root_period_bytes: Option<swagger::ByteArray>,
        checkpoint_period_log_length: Option<i64>,
        checkpoint_period_map_root_period_algo: Option<String>,
        checkpoint_period_map_root_period_bytes: Option<swagger::ByteArray>,
        ) -> Result<WargProveInclusionResponse, ApiError>;

}

/// Trait to extend an API to make it easy to bind it to a context.
pub trait ContextWrapperExt<C: Send + Sync> where Self: Sized
{
    /// Binds this API to a context.
    fn with_context(self, context: C) -> ContextWrapper<Self, C>;
}

impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ContextWrapperExt<C> for T {
    fn with_context(self: T, context: C) -> ContextWrapper<T, C> {
         ContextWrapper::<T, C>::new(self, context)
    }
}

#[async_trait]
impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ApiNoContext<C> for ContextWrapper<T, C> {
    fn poll_ready(&self, cx: &mut Context) -> Poll<Result<(), ServiceError>> {
        self.api().poll_ready(cx)
    }

    fn context(&self) -> &C {
        ContextWrapper::context(self)
    }

    /// Fetches logs for a requested package.
    async fn warg_fetch_logs(
        &self,
        root_period_algo: Option<String>,
        root_period_bytes: Option<swagger::ByteArray>,
        operator_period_algo: Option<String>,
        operator_period_bytes: Option<swagger::ByteArray>,
        ) -> Result<WargFetchLogsResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().warg_fetch_logs(root_period_algo, root_period_bytes, operator_period_algo, operator_period_bytes, &context).await
    }

    /// Used for polling while package is being in the processed of publishing.
    async fn warg_get_package(
        &self,
        package_id: String,
        ) -> Result<WargGetPackageResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().warg_get_package(package_id, &context).await
    }

    /// Get a specific record within a package.
    async fn warg_get_package_record(
        &self,
        package_id: String,
        record_id: String,
        ) -> Result<WargGetPackageRecordResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().warg_get_package_record(package_id, record_id, &context).await
    }

    /// Request that a new package be published.
    async fn warg_publish_package(
        &self,
        name: Option<String>,
        record_period_contents: Option<swagger::ByteArray>,
        record_period_key_id: Option<String>,
        record_period_signature: Option<String>,
        ) -> Result<WargPublishPackageResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().warg_publish_package(name, record_period_contents, record_period_key_id, record_period_signature, &context).await
    }

    /// Fetches logs for a root.
    async fn warg_fetch_checkpoint(
        &self,
        ) -> Result<WargFetchCheckpointResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().warg_fetch_checkpoint(&context).await
    }

    /// Proves consistency between an old root and a new one.
    async fn warg_prove_consistency(
        &self,
        old_root_period_algo: Option<String>,
        old_root_period_bytes: Option<swagger::ByteArray>,
        new_root_period_algo: Option<String>,
        new_root_period_bytes: Option<swagger::ByteArray>,
        ) -> Result<WargProveConsistencyResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().warg_prove_consistency(old_root_period_algo, old_root_period_bytes, new_root_period_algo, new_root_period_bytes, &context).await
    }

    /// Proves inclusion between a log and a map.
    async fn warg_prove_inclusion(
        &self,
        checkpoint_period_log_root_period_algo: Option<String>,
        checkpoint_period_log_root_period_bytes: Option<swagger::ByteArray>,
        checkpoint_period_log_length: Option<i64>,
        checkpoint_period_map_root_period_algo: Option<String>,
        checkpoint_period_map_root_period_bytes: Option<swagger::ByteArray>,
        ) -> Result<WargProveInclusionResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().warg_prove_inclusion(checkpoint_period_log_root_period_algo, checkpoint_period_log_root_period_bytes, checkpoint_period_log_length, checkpoint_period_map_root_period_algo, checkpoint_period_map_root_period_bytes, &context).await
    }

}


#[cfg(feature = "client")]
pub mod client;

// Re-export Client as a top-level name
#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub mod server;

// Re-export router() as a top-level name
#[cfg(feature = "server")]
pub use self::server::Service;

#[cfg(feature = "server")]
pub mod context;

pub mod models;

#[cfg(any(feature = "client", feature = "server"))]
pub(crate) mod header;
