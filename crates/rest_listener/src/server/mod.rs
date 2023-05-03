use futures::{future, future::BoxFuture, Stream, stream, future::FutureExt, stream::TryStreamExt};
use hyper::{Request, Response, StatusCode, Body, HeaderMap};
use hyper::header::{HeaderName, HeaderValue, CONTENT_TYPE};
use log::warn;
#[allow(unused_imports)]
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::future::Future;
use std::marker::PhantomData;
use std::task::{Context, Poll};
use swagger::{ApiError, BodyExt, Has, RequestParser, XSpanIdString};
pub use swagger::auth::Authorization;
use swagger::auth::Scopes;
use url::form_urlencoded;

#[allow(unused_imports)]
use crate::models;
use crate::header;

pub use crate::context;

type ServiceFuture = BoxFuture<'static, Result<Response<Body>, crate::ServiceError>>;

use crate::{Api,
     WargFetchLogsResponse,
     WargGetPackageResponse,
     WargGetPackageRecordResponse,
     WargPublishPackageResponse,
     WargFetchCheckpointResponse,
     WargProveConsistencyResponse,
     WargProveInclusionResponse
};

mod paths {
    use lazy_static::lazy_static;

    lazy_static! {
        pub static ref GLOBAL_REGEX_SET: regex::RegexSet = regex::RegexSet::new(vec![
            r"^/checkpoint/fetch$",
            r"^/logs/fetch$",
            r"^/package$",
            r"^/package/(?P<packageId>[^/?#]*)$",
            r"^/package/(?P<packageId>[^/?#]*)/records/(?P<recordId>[^/?#]*)$",
            r"^/prove/consistency$",
            r"^/prove/inclusion$"
        ])
        .expect("Unable to create global regex set");
    }
    pub(crate) static ID_CHECKPOINT_FETCH: usize = 0;
    pub(crate) static ID_LOGS_FETCH: usize = 1;
    pub(crate) static ID_PACKAGE: usize = 2;
    pub(crate) static ID_PACKAGE_PACKAGEID: usize = 3;
    lazy_static! {
        pub static ref REGEX_PACKAGE_PACKAGEID: regex::Regex =
            #[allow(clippy::invalid_regex)]
            regex::Regex::new(r"^/package/(?P<packageId>[^/?#]*)$")
                .expect("Unable to create regex for PACKAGE_PACKAGEID");
    }
    pub(crate) static ID_PACKAGE_PACKAGEID_RECORDS_RECORDID: usize = 4;
    lazy_static! {
        pub static ref REGEX_PACKAGE_PACKAGEID_RECORDS_RECORDID: regex::Regex =
            #[allow(clippy::invalid_regex)]
            regex::Regex::new(r"^/package/(?P<packageId>[^/?#]*)/records/(?P<recordId>[^/?#]*)$")
                .expect("Unable to create regex for PACKAGE_PACKAGEID_RECORDS_RECORDID");
    }
    pub(crate) static ID_PROVE_CONSISTENCY: usize = 5;
    pub(crate) static ID_PROVE_INCLUSION: usize = 6;
}

pub struct MakeService<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    api_impl: T,
    marker: PhantomData<C>,
}

impl<T, C> MakeService<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    pub fn new(api_impl: T) -> Self {
        MakeService {
            api_impl,
            marker: PhantomData
        }
    }
}

impl<T, C, Target> hyper::service::Service<Target> for MakeService<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    type Response = Service<T, C>;
    type Error = crate::ServiceError;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, target: Target) -> Self::Future {
        futures::future::ok(Service::new(
            self.api_impl.clone(),
        ))
    }
}

fn method_not_allowed() -> Result<Response<Body>, crate::ServiceError> {
    Ok(
        Response::builder().status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::empty())
            .expect("Unable to create Method Not Allowed response")
    )
}

pub struct Service<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    api_impl: T,
    marker: PhantomData<C>,
}

impl<T, C> Service<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    pub fn new(api_impl: T) -> Self {
        Service {
            api_impl,
            marker: PhantomData
        }
    }
}

impl<T, C> Clone for Service<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    fn clone(&self) -> Self {
        Service {
            api_impl: self.api_impl.clone(),
            marker: self.marker,
        }
    }
}

impl<T, C> hyper::service::Service<(Request<Body>, C)> for Service<T, C> where
    T: Api<C> + Clone + Send + Sync + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    type Response = Response<Body>;
    type Error = crate::ServiceError;
    type Future = ServiceFuture;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.api_impl.poll_ready(cx)
    }

    fn call(&mut self, req: (Request<Body>, C)) -> Self::Future { async fn run<T, C>(mut api_impl: T, req: (Request<Body>, C)) -> Result<Response<Body>, crate::ServiceError> where
        T: Api<C> + Clone + Send + 'static,
        C: Has<XSpanIdString>  + Send + Sync + 'static
    {
        let (request, context) = req;
        let (parts, body) = request.into_parts();
        let (method, uri, headers) = (parts.method, parts.uri, parts.headers);
        let path = paths::GLOBAL_REGEX_SET.matches(uri.path());

        match method {

            // WargFetchLogs - POST /logs/fetch
            hyper::Method::POST if path.matched(paths::ID_LOGS_FETCH) => {
                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_root_period_algo = query_params.iter().filter(|e| e.0 == "root.algo").map(|e| e.1.clone())
                    .next();
                let param_root_period_algo = match param_root_period_algo {
                    Some(param_root_period_algo) => {
                        let param_root_period_algo =
                            <String as std::str::FromStr>::from_str
                                (&param_root_period_algo);
                        match param_root_period_algo {
                            Ok(param_root_period_algo) => Some(param_root_period_algo),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter root.algo - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter root.algo")),
                        }
                    },
                    None => None,
                };
                let param_root_period_bytes = query_params.iter().filter(|e| e.0 == "root.bytes").map(|e| e.1.clone())
                    .next();
                let param_root_period_bytes = match param_root_period_bytes {
                    Some(param_root_period_bytes) => {
                        let param_root_period_bytes =
                            <swagger::ByteArray as std::str::FromStr>::from_str
                                (&param_root_period_bytes);
                        match param_root_period_bytes {
                            Ok(param_root_period_bytes) => Some(param_root_period_bytes),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter root.bytes - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter root.bytes")),
                        }
                    },
                    None => None,
                };
                let param_operator_period_algo = query_params.iter().filter(|e| e.0 == "operator.algo").map(|e| e.1.clone())
                    .next();
                let param_operator_period_algo = match param_operator_period_algo {
                    Some(param_operator_period_algo) => {
                        let param_operator_period_algo =
                            <String as std::str::FromStr>::from_str
                                (&param_operator_period_algo);
                        match param_operator_period_algo {
                            Ok(param_operator_period_algo) => Some(param_operator_period_algo),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter operator.algo - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter operator.algo")),
                        }
                    },
                    None => None,
                };
                let param_operator_period_bytes = query_params.iter().filter(|e| e.0 == "operator.bytes").map(|e| e.1.clone())
                    .next();
                let param_operator_period_bytes = match param_operator_period_bytes {
                    Some(param_operator_period_bytes) => {
                        let param_operator_period_bytes =
                            <swagger::ByteArray as std::str::FromStr>::from_str
                                (&param_operator_period_bytes);
                        match param_operator_period_bytes {
                            Ok(param_operator_period_bytes) => Some(param_operator_period_bytes),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter operator.bytes - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter operator.bytes")),
                        }
                    },
                    None => None,
                };

                                let result = api_impl.warg_fetch_logs(
                                            param_root_period_algo,
                                            param_root_period_bytes,
                                            param_operator_period_algo,
                                            param_operator_period_bytes,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                WargFetchLogsResponse::ASuccessfulResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_FETCH_LOGS_A_SUCCESSFUL_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                WargFetchLogsResponse::AnUnexpectedErrorResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(0).expect("Unable to turn 0 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_FETCH_LOGS_AN_UNEXPECTED_ERROR_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // WargGetPackage - GET /package/{packageId}
            hyper::Method::GET if path.matched(paths::ID_PACKAGE_PACKAGEID) => {
                // Path parameters
                let path: &str = uri.path();
                let path_params =
                    paths::REGEX_PACKAGE_PACKAGEID
                    .captures(path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE PACKAGE_PACKAGEID in set but failed match against \"{}\"", path, paths::REGEX_PACKAGE_PACKAGEID.as_str())
                    );

                let param_package_id = match percent_encoding::percent_decode(path_params["packageId"].as_bytes()).decode_utf8() {
                    Ok(param_package_id) => match param_package_id.parse::<String>() {
                        Ok(param_package_id) => param_package_id,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter packageId: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["packageId"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                                let result = api_impl.warg_get_package(
                                            param_package_id,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                WargGetPackageResponse::ASuccessfulResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_GET_PACKAGE_A_SUCCESSFUL_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                WargGetPackageResponse::AnUnexpectedErrorResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(0).expect("Unable to turn 0 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_GET_PACKAGE_AN_UNEXPECTED_ERROR_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // WargGetPackageRecord - GET /package/{packageId}/records/{recordId}
            hyper::Method::GET if path.matched(paths::ID_PACKAGE_PACKAGEID_RECORDS_RECORDID) => {
                // Path parameters
                let path: &str = uri.path();
                let path_params =
                    paths::REGEX_PACKAGE_PACKAGEID_RECORDS_RECORDID
                    .captures(path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE PACKAGE_PACKAGEID_RECORDS_RECORDID in set but failed match against \"{}\"", path, paths::REGEX_PACKAGE_PACKAGEID_RECORDS_RECORDID.as_str())
                    );

                let param_package_id = match percent_encoding::percent_decode(path_params["packageId"].as_bytes()).decode_utf8() {
                    Ok(param_package_id) => match param_package_id.parse::<String>() {
                        Ok(param_package_id) => param_package_id,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter packageId: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["packageId"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                let param_record_id = match percent_encoding::percent_decode(path_params["recordId"].as_bytes()).decode_utf8() {
                    Ok(param_record_id) => match param_record_id.parse::<String>() {
                        Ok(param_record_id) => param_record_id,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter recordId: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["recordId"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                                let result = api_impl.warg_get_package_record(
                                            param_package_id,
                                            param_record_id,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                WargGetPackageRecordResponse::ASuccessfulResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_GET_PACKAGE_RECORD_A_SUCCESSFUL_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                WargGetPackageRecordResponse::AnUnexpectedErrorResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(0).expect("Unable to turn 0 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_GET_PACKAGE_RECORD_AN_UNEXPECTED_ERROR_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // WargPublishPackage - POST /package
            hyper::Method::POST if path.matched(paths::ID_PACKAGE) => {
                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_name = query_params.iter().filter(|e| e.0 == "name").map(|e| e.1.clone())
                    .next();
                let param_name = match param_name {
                    Some(param_name) => {
                        let param_name =
                            <String as std::str::FromStr>::from_str
                                (&param_name);
                        match param_name {
                            Ok(param_name) => Some(param_name),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter name - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter name")),
                        }
                    },
                    None => None,
                };
                let param_record_period_contents = query_params.iter().filter(|e| e.0 == "record.contents").map(|e| e.1.clone())
                    .next();
                let param_record_period_contents = match param_record_period_contents {
                    Some(param_record_period_contents) => {
                        let param_record_period_contents =
                            <swagger::ByteArray as std::str::FromStr>::from_str
                                (&param_record_period_contents);
                        match param_record_period_contents {
                            Ok(param_record_period_contents) => Some(param_record_period_contents),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter record.contents - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter record.contents")),
                        }
                    },
                    None => None,
                };
                let param_record_period_key_id = query_params.iter().filter(|e| e.0 == "record.keyId").map(|e| e.1.clone())
                    .next();
                let param_record_period_key_id = match param_record_period_key_id {
                    Some(param_record_period_key_id) => {
                        let param_record_period_key_id =
                            <String as std::str::FromStr>::from_str
                                (&param_record_period_key_id);
                        match param_record_period_key_id {
                            Ok(param_record_period_key_id) => Some(param_record_period_key_id),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter record.keyId - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter record.keyId")),
                        }
                    },
                    None => None,
                };
                let param_record_period_signature = query_params.iter().filter(|e| e.0 == "record.signature").map(|e| e.1.clone())
                    .next();
                let param_record_period_signature = match param_record_period_signature {
                    Some(param_record_period_signature) => {
                        let param_record_period_signature =
                            <String as std::str::FromStr>::from_str
                                (&param_record_period_signature);
                        match param_record_period_signature {
                            Ok(param_record_period_signature) => Some(param_record_period_signature),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter record.signature - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter record.signature")),
                        }
                    },
                    None => None,
                };

                                let result = api_impl.warg_publish_package(
                                            param_name,
                                            param_record_period_contents,
                                            param_record_period_key_id,
                                            param_record_period_signature,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                WargPublishPackageResponse::ASuccessfulResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_PUBLISH_PACKAGE_A_SUCCESSFUL_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                WargPublishPackageResponse::AnUnexpectedErrorResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(0).expect("Unable to turn 0 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_PUBLISH_PACKAGE_AN_UNEXPECTED_ERROR_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // WargFetchCheckpoint - POST /checkpoint/fetch
            hyper::Method::POST if path.matched(paths::ID_CHECKPOINT_FETCH) => {
                                let result = api_impl.warg_fetch_checkpoint(
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                WargFetchCheckpointResponse::ASuccessfulResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_FETCH_CHECKPOINT_A_SUCCESSFUL_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                WargFetchCheckpointResponse::AnUnexpectedErrorResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(0).expect("Unable to turn 0 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_FETCH_CHECKPOINT_AN_UNEXPECTED_ERROR_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // WargProveConsistency - POST /prove/consistency
            hyper::Method::POST if path.matched(paths::ID_PROVE_CONSISTENCY) => {
                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_old_root_period_algo = query_params.iter().filter(|e| e.0 == "oldRoot.algo").map(|e| e.1.clone())
                    .next();
                let param_old_root_period_algo = match param_old_root_period_algo {
                    Some(param_old_root_period_algo) => {
                        let param_old_root_period_algo =
                            <String as std::str::FromStr>::from_str
                                (&param_old_root_period_algo);
                        match param_old_root_period_algo {
                            Ok(param_old_root_period_algo) => Some(param_old_root_period_algo),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter oldRoot.algo - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter oldRoot.algo")),
                        }
                    },
                    None => None,
                };
                let param_old_root_period_bytes = query_params.iter().filter(|e| e.0 == "oldRoot.bytes").map(|e| e.1.clone())
                    .next();
                let param_old_root_period_bytes = match param_old_root_period_bytes {
                    Some(param_old_root_period_bytes) => {
                        let param_old_root_period_bytes =
                            <swagger::ByteArray as std::str::FromStr>::from_str
                                (&param_old_root_period_bytes);
                        match param_old_root_period_bytes {
                            Ok(param_old_root_period_bytes) => Some(param_old_root_period_bytes),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter oldRoot.bytes - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter oldRoot.bytes")),
                        }
                    },
                    None => None,
                };
                let param_new_root_period_algo = query_params.iter().filter(|e| e.0 == "newRoot.algo").map(|e| e.1.clone())
                    .next();
                let param_new_root_period_algo = match param_new_root_period_algo {
                    Some(param_new_root_period_algo) => {
                        let param_new_root_period_algo =
                            <String as std::str::FromStr>::from_str
                                (&param_new_root_period_algo);
                        match param_new_root_period_algo {
                            Ok(param_new_root_period_algo) => Some(param_new_root_period_algo),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter newRoot.algo - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter newRoot.algo")),
                        }
                    },
                    None => None,
                };
                let param_new_root_period_bytes = query_params.iter().filter(|e| e.0 == "newRoot.bytes").map(|e| e.1.clone())
                    .next();
                let param_new_root_period_bytes = match param_new_root_period_bytes {
                    Some(param_new_root_period_bytes) => {
                        let param_new_root_period_bytes =
                            <swagger::ByteArray as std::str::FromStr>::from_str
                                (&param_new_root_period_bytes);
                        match param_new_root_period_bytes {
                            Ok(param_new_root_period_bytes) => Some(param_new_root_period_bytes),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter newRoot.bytes - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter newRoot.bytes")),
                        }
                    },
                    None => None,
                };

                                let result = api_impl.warg_prove_consistency(
                                            param_old_root_period_algo,
                                            param_old_root_period_bytes,
                                            param_new_root_period_algo,
                                            param_new_root_period_bytes,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                WargProveConsistencyResponse::ASuccessfulResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_PROVE_CONSISTENCY_A_SUCCESSFUL_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                WargProveConsistencyResponse::AnUnexpectedErrorResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(0).expect("Unable to turn 0 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_PROVE_CONSISTENCY_AN_UNEXPECTED_ERROR_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // WargProveInclusion - POST /prove/inclusion
            hyper::Method::POST if path.matched(paths::ID_PROVE_INCLUSION) => {
                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_checkpoint_period_log_root_period_algo = query_params.iter().filter(|e| e.0 == "checkpoint.logRoot.algo").map(|e| e.1.clone())
                    .next();
                let param_checkpoint_period_log_root_period_algo = match param_checkpoint_period_log_root_period_algo {
                    Some(param_checkpoint_period_log_root_period_algo) => {
                        let param_checkpoint_period_log_root_period_algo =
                            <String as std::str::FromStr>::from_str
                                (&param_checkpoint_period_log_root_period_algo);
                        match param_checkpoint_period_log_root_period_algo {
                            Ok(param_checkpoint_period_log_root_period_algo) => Some(param_checkpoint_period_log_root_period_algo),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter checkpoint.logRoot.algo - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter checkpoint.logRoot.algo")),
                        }
                    },
                    None => None,
                };
                let param_checkpoint_period_log_root_period_bytes = query_params.iter().filter(|e| e.0 == "checkpoint.logRoot.bytes").map(|e| e.1.clone())
                    .next();
                let param_checkpoint_period_log_root_period_bytes = match param_checkpoint_period_log_root_period_bytes {
                    Some(param_checkpoint_period_log_root_period_bytes) => {
                        let param_checkpoint_period_log_root_period_bytes =
                            <swagger::ByteArray as std::str::FromStr>::from_str
                                (&param_checkpoint_period_log_root_period_bytes);
                        match param_checkpoint_period_log_root_period_bytes {
                            Ok(param_checkpoint_period_log_root_period_bytes) => Some(param_checkpoint_period_log_root_period_bytes),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter checkpoint.logRoot.bytes - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter checkpoint.logRoot.bytes")),
                        }
                    },
                    None => None,
                };
                let param_checkpoint_period_log_length = query_params.iter().filter(|e| e.0 == "checkpoint.logLength").map(|e| e.1.clone())
                    .next();
                let param_checkpoint_period_log_length = match param_checkpoint_period_log_length {
                    Some(param_checkpoint_period_log_length) => {
                        let param_checkpoint_period_log_length =
                            <i64 as std::str::FromStr>::from_str
                                (&param_checkpoint_period_log_length);
                        match param_checkpoint_period_log_length {
                            Ok(param_checkpoint_period_log_length) => Some(param_checkpoint_period_log_length),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter checkpoint.logLength - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter checkpoint.logLength")),
                        }
                    },
                    None => None,
                };
                let param_checkpoint_period_map_root_period_algo = query_params.iter().filter(|e| e.0 == "checkpoint.mapRoot.algo").map(|e| e.1.clone())
                    .next();
                let param_checkpoint_period_map_root_period_algo = match param_checkpoint_period_map_root_period_algo {
                    Some(param_checkpoint_period_map_root_period_algo) => {
                        let param_checkpoint_period_map_root_period_algo =
                            <String as std::str::FromStr>::from_str
                                (&param_checkpoint_period_map_root_period_algo);
                        match param_checkpoint_period_map_root_period_algo {
                            Ok(param_checkpoint_period_map_root_period_algo) => Some(param_checkpoint_period_map_root_period_algo),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter checkpoint.mapRoot.algo - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter checkpoint.mapRoot.algo")),
                        }
                    },
                    None => None,
                };
                let param_checkpoint_period_map_root_period_bytes = query_params.iter().filter(|e| e.0 == "checkpoint.mapRoot.bytes").map(|e| e.1.clone())
                    .next();
                let param_checkpoint_period_map_root_period_bytes = match param_checkpoint_period_map_root_period_bytes {
                    Some(param_checkpoint_period_map_root_period_bytes) => {
                        let param_checkpoint_period_map_root_period_bytes =
                            <swagger::ByteArray as std::str::FromStr>::from_str
                                (&param_checkpoint_period_map_root_period_bytes);
                        match param_checkpoint_period_map_root_period_bytes {
                            Ok(param_checkpoint_period_map_root_period_bytes) => Some(param_checkpoint_period_map_root_period_bytes),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter checkpoint.mapRoot.bytes - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter checkpoint.mapRoot.bytes")),
                        }
                    },
                    None => None,
                };

                                let result = api_impl.warg_prove_inclusion(
                                            param_checkpoint_period_log_root_period_algo,
                                            param_checkpoint_period_log_root_period_bytes,
                                            param_checkpoint_period_log_length,
                                            param_checkpoint_period_map_root_period_algo,
                                            param_checkpoint_period_map_root_period_bytes,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                WargProveInclusionResponse::ASuccessfulResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_PROVE_INCLUSION_A_SUCCESSFUL_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                                WargProveInclusionResponse::AnUnexpectedErrorResponse
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(0).expect("Unable to turn 0 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for WARG_PROVE_INCLUSION_AN_UNEXPECTED_ERROR_RESPONSE"));
                                                    let body = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            _ if path.matched(paths::ID_CHECKPOINT_FETCH) => method_not_allowed(),
            _ if path.matched(paths::ID_LOGS_FETCH) => method_not_allowed(),
            _ if path.matched(paths::ID_PACKAGE) => method_not_allowed(),
            _ if path.matched(paths::ID_PACKAGE_PACKAGEID) => method_not_allowed(),
            _ if path.matched(paths::ID_PACKAGE_PACKAGEID_RECORDS_RECORDID) => method_not_allowed(),
            _ if path.matched(paths::ID_PROVE_CONSISTENCY) => method_not_allowed(),
            _ if path.matched(paths::ID_PROVE_INCLUSION) => method_not_allowed(),
            _ => Ok(Response::builder().status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .expect("Unable to create Not Found response"))
        }
    } Box::pin(run(self.api_impl.clone(), req)) }
}

/// Request parser for `Api`.
pub struct ApiRequestParser;
impl<T> RequestParser<T> for ApiRequestParser {
    fn parse_operation_id(request: &Request<T>) -> Option<&'static str> {
        let path = paths::GLOBAL_REGEX_SET.matches(request.uri().path());
        match *request.method() {
            // WargFetchLogs - POST /logs/fetch
            hyper::Method::POST if path.matched(paths::ID_LOGS_FETCH) => Some("WargFetchLogs"),
            // WargGetPackage - GET /package/{packageId}
            hyper::Method::GET if path.matched(paths::ID_PACKAGE_PACKAGEID) => Some("WargGetPackage"),
            // WargGetPackageRecord - GET /package/{packageId}/records/{recordId}
            hyper::Method::GET if path.matched(paths::ID_PACKAGE_PACKAGEID_RECORDS_RECORDID) => Some("WargGetPackageRecord"),
            // WargPublishPackage - POST /package
            hyper::Method::POST if path.matched(paths::ID_PACKAGE) => Some("WargPublishPackage"),
            // WargFetchCheckpoint - POST /checkpoint/fetch
            hyper::Method::POST if path.matched(paths::ID_CHECKPOINT_FETCH) => Some("WargFetchCheckpoint"),
            // WargProveConsistency - POST /prove/consistency
            hyper::Method::POST if path.matched(paths::ID_PROVE_CONSISTENCY) => Some("WargProveConsistency"),
            // WargProveInclusion - POST /prove/inclusion
            hyper::Method::POST if path.matched(paths::ID_PROVE_INCLUSION) => Some("WargProveInclusion"),
            _ => None,
        }
    }
}
