/// Represents an HTTP request.
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HttpRequest {
    /// The HTTP request method.
    #[prost(string, tag = "1")]
    pub method: ::prost::alloc::string::String,
    /// The HTTP request URI.
    #[prost(string, tag = "2")]
    pub uri: ::prost::alloc::string::String,
    /// The HTTP request headers. The ordering of the headers is significant.
    /// Multiple headers with the same key may present for the request.
    #[prost(message, repeated, tag = "3")]
    pub headers: ::prost::alloc::vec::Vec<HttpHeader>,
    /// The HTTP request body. If the body is not expected, it should be empty.
    #[prost(bytes = "vec", tag = "4")]
    pub body: ::prost::alloc::vec::Vec<u8>,
}
/// Represents an HTTP response.
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HttpResponse {
    /// The HTTP status code, such as 200 or 404.
    #[prost(int32, tag = "1")]
    pub status: i32,
    /// The HTTP reason phrase, such as "OK" or "Not Found".
    #[prost(string, tag = "2")]
    pub reason: ::prost::alloc::string::String,
    /// The HTTP response headers. The ordering of the headers is significant.
    /// Multiple headers with the same key may present for the response.
    #[prost(message, repeated, tag = "3")]
    pub headers: ::prost::alloc::vec::Vec<HttpHeader>,
    /// The HTTP response body. If the body is not expected, it should be empty.
    #[prost(bytes = "vec", tag = "4")]
    pub body: ::prost::alloc::vec::Vec<u8>,
}
/// Represents an HTTP header.
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HttpHeader {
    /// The HTTP header key. It is case insensitive.
    #[prost(string, tag = "1")]
    pub key: ::prost::alloc::string::String,
    /// The HTTP header value.
    #[prost(string, tag = "2")]
    pub value: ::prost::alloc::string::String,
}
