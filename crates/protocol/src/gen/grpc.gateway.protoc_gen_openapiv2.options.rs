/// `Swagger` is a representation of OpenAPI v2 specification's Swagger object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#swaggerObject>
///
/// Example:
///
/// option (grpc.gateway.protoc_gen_openapiv2.options.openapiv2_swagger) = {
/// info: {
/// title: "Echo API";
/// version: "1.0";
/// description: "";
/// contact: {
/// name: "gRPC-Gateway project";
/// url: "<https://github.com/grpc-ecosystem/grpc-gateway";>
/// email: "none@example.com";
/// };
/// license: {
/// name: "BSD 3-Clause License";
/// url: "<https://github.com/grpc-ecosystem/grpc-gateway/blob/main/LICENSE.txt";>
/// };
/// };
/// schemes: HTTPS;
/// consumes: "application/json";
/// produces: "application/json";
/// };
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Swagger {
    /// Specifies the OpenAPI Specification version being used. It can be
    /// used by the OpenAPI UI and other clients to interpret the API listing. The
    /// value MUST be "2.0".
    #[prost(string, tag = "1")]
    pub swagger: ::prost::alloc::string::String,
    /// Provides metadata about the API. The metadata can be used by the
    /// clients if needed.
    #[prost(message, optional, tag = "2")]
    pub info: ::core::option::Option<Info>,
    /// The host (name or ip) serving the API. This MUST be the host only and does
    /// not include the scheme nor sub-paths. It MAY include a port. If the host is
    /// not included, the host serving the documentation is to be used (including
    /// the port). The host does not support path templating.
    #[prost(string, tag = "3")]
    pub host: ::prost::alloc::string::String,
    /// The base path on which the API is served, which is relative to the host. If
    /// it is not included, the API is served directly under the host. The value
    /// MUST start with a leading slash (/). The basePath does not support path
    /// templating.
    /// Note that using `base_path` does not change the endpoint paths that are
    /// generated in the resulting OpenAPI file. If you wish to use `base_path`
    /// with relatively generated OpenAPI paths, the `base_path` prefix must be
    /// manually removed from your `google.api.http` paths and your code changed to
    /// serve the API from the `base_path`.
    #[prost(string, tag = "4")]
    pub base_path: ::prost::alloc::string::String,
    /// The transfer protocol of the API. Values MUST be from the list: "http",
    /// "https", "ws", "wss". If the schemes is not included, the default scheme to
    /// be used is the one used to access the OpenAPI definition itself.
    #[prost(enumeration = "Scheme", repeated, tag = "5")]
    pub schemes: ::prost::alloc::vec::Vec<i32>,
    /// A list of MIME types the APIs can consume. This is global to all APIs but
    /// can be overridden on specific API calls. Value MUST be as described under
    /// Mime Types.
    #[prost(string, repeated, tag = "6")]
    pub consumes: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// A list of MIME types the APIs can produce. This is global to all APIs but
    /// can be overridden on specific API calls. Value MUST be as described under
    /// Mime Types.
    #[prost(string, repeated, tag = "7")]
    pub produces: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// An object to hold responses that can be used across operations. This
    /// property does not define global responses for all operations.
    #[prost(map = "string, message", tag = "10")]
    pub responses: ::std::collections::HashMap<::prost::alloc::string::String, Response>,
    /// Security scheme definitions that can be used across the specification.
    #[prost(message, optional, tag = "11")]
    pub security_definitions: ::core::option::Option<SecurityDefinitions>,
    /// A declaration of which security schemes are applied for the API as a whole.
    /// The list of values describes alternative security schemes that can be used
    /// (that is, there is a logical OR between the security requirements).
    /// Individual operations can override this definition.
    #[prost(message, repeated, tag = "12")]
    pub security: ::prost::alloc::vec::Vec<SecurityRequirement>,
    /// A list of tags for API documentation control. Tags can be used for logical
    /// grouping of operations by resources or any other qualifier.
    #[prost(message, repeated, tag = "13")]
    pub tags: ::prost::alloc::vec::Vec<Tag>,
    /// Additional external documentation.
    #[prost(message, optional, tag = "14")]
    pub external_docs: ::core::option::Option<ExternalDocumentation>,
    /// Custom properties that start with "x-" such as "x-foo" used to describe
    /// extra functionality that is not covered by the standard OpenAPI Specification.
    /// See: <https://swagger.io/docs/specification/2-0/swagger-extensions/>
    #[prost(map = "string, message", tag = "15")]
    pub extensions: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost_types::Value,
    >,
}
/// `Operation` is a representation of OpenAPI v2 specification's Operation object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#operationObject>
///
/// Example:
///
/// service EchoService {
/// rpc Echo(SimpleMessage) returns (SimpleMessage) {
/// option (google.api.http) = {
/// get: "/v1/example/echo/{id}"
/// };
///
/// ```text
///   option (grpc.gateway.protoc_gen_openapiv2.options.openapiv2_operation) = {
///     summary: "Get a message.";
///     operation_id: "getMessage";
///     tags: "echo";
///     responses: {
///       key: "200"
///         value: {
///         description: "OK";
///       }
///     }
///   };
/// }
/// ```
///
/// }
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Operation {
    /// A list of tags for API documentation control. Tags can be used for logical
    /// grouping of operations by resources or any other qualifier.
    #[prost(string, repeated, tag = "1")]
    pub tags: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// A short summary of what the operation does. For maximum readability in the
    /// swagger-ui, this field SHOULD be less than 120 characters.
    #[prost(string, tag = "2")]
    pub summary: ::prost::alloc::string::String,
    /// A verbose explanation of the operation behavior. GFM syntax can be used for
    /// rich text representation.
    #[prost(string, tag = "3")]
    pub description: ::prost::alloc::string::String,
    /// Additional external documentation for this operation.
    #[prost(message, optional, tag = "4")]
    pub external_docs: ::core::option::Option<ExternalDocumentation>,
    /// Unique string used to identify the operation. The id MUST be unique among
    /// all operations described in the API. Tools and libraries MAY use the
    /// operationId to uniquely identify an operation, therefore, it is recommended
    /// to follow common programming naming conventions.
    #[prost(string, tag = "5")]
    pub operation_id: ::prost::alloc::string::String,
    /// A list of MIME types the operation can consume. This overrides the consumes
    /// definition at the OpenAPI Object. An empty value MAY be used to clear the
    /// global definition. Value MUST be as described under Mime Types.
    #[prost(string, repeated, tag = "6")]
    pub consumes: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// A list of MIME types the operation can produce. This overrides the produces
    /// definition at the OpenAPI Object. An empty value MAY be used to clear the
    /// global definition. Value MUST be as described under Mime Types.
    #[prost(string, repeated, tag = "7")]
    pub produces: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// The list of possible responses as they are returned from executing this
    /// operation.
    #[prost(map = "string, message", tag = "9")]
    pub responses: ::std::collections::HashMap<::prost::alloc::string::String, Response>,
    /// The transfer protocol for the operation. Values MUST be from the list:
    /// "http", "https", "ws", "wss". The value overrides the OpenAPI Object
    /// schemes definition.
    #[prost(enumeration = "Scheme", repeated, tag = "10")]
    pub schemes: ::prost::alloc::vec::Vec<i32>,
    /// Declares this operation to be deprecated. Usage of the declared operation
    /// should be refrained. Default value is false.
    #[prost(bool, tag = "11")]
    pub deprecated: bool,
    /// A declaration of which security schemes are applied for this operation. The
    /// list of values describes alternative security schemes that can be used
    /// (that is, there is a logical OR between the security requirements). This
    /// definition overrides any declared top-level security. To remove a top-level
    /// security declaration, an empty array can be used.
    #[prost(message, repeated, tag = "12")]
    pub security: ::prost::alloc::vec::Vec<SecurityRequirement>,
    /// Custom properties that start with "x-" such as "x-foo" used to describe
    /// extra functionality that is not covered by the standard OpenAPI Specification.
    /// See: <https://swagger.io/docs/specification/2-0/swagger-extensions/>
    #[prost(map = "string, message", tag = "13")]
    pub extensions: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost_types::Value,
    >,
    /// Custom parameters such as HTTP request headers.
    /// See: <https://swagger.io/docs/specification/2-0/describing-parameters/>
    /// and <https://swagger.io/specification/v2/#parameter-object.>
    #[prost(message, optional, tag = "14")]
    pub parameters: ::core::option::Option<Parameters>,
}
/// `Parameters` is a representation of OpenAPI v2 specification's parameters object.
/// Note: This technically breaks compatibility with the OpenAPI 2 definition structure as we only
/// allow header parameters to be set here since we do not want users specifying custom non-header
/// parameters beyond those inferred from the Protobuf schema.
/// See: <https://swagger.io/specification/v2/#parameter-object>
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Parameters {
    /// `Headers` is one or more HTTP header parameter.
    /// See: <https://swagger.io/docs/specification/2-0/describing-parameters/#header-parameters>
    #[prost(message, repeated, tag = "1")]
    pub headers: ::prost::alloc::vec::Vec<HeaderParameter>,
}
/// `HeaderParameter` a HTTP header parameter.
/// See: <https://swagger.io/specification/v2/#parameter-object>
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HeaderParameter {
    /// `Name` is the header name.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// `Description` is a short description of the header.
    #[prost(string, tag = "2")]
    pub description: ::prost::alloc::string::String,
    /// `Type` is the type of the object. The value MUST be one of "string", "number", "integer", or "boolean". The "array" type is not supported.
    /// See: <https://swagger.io/specification/v2/#parameterType.>
    #[prost(enumeration = "header_parameter::Type", tag = "3")]
    pub r#type: i32,
    /// `Format` The extending format for the previously mentioned type.
    #[prost(string, tag = "4")]
    pub format: ::prost::alloc::string::String,
    /// `Required` indicates if the header is optional
    #[prost(bool, tag = "5")]
    pub required: bool,
}
/// Nested message and enum types in `HeaderParameter`.
pub mod header_parameter {
    /// `Type` is a a supported HTTP header type.
    /// See <https://swagger.io/specification/v2/#parameterType.>
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Type {
        Unknown = 0,
        String = 1,
        Number = 2,
        Integer = 3,
        Boolean = 4,
    }
    impl Type {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Type::Unknown => "UNKNOWN",
                Type::String => "STRING",
                Type::Number => "NUMBER",
                Type::Integer => "INTEGER",
                Type::Boolean => "BOOLEAN",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "UNKNOWN" => Some(Self::Unknown),
                "STRING" => Some(Self::String),
                "NUMBER" => Some(Self::Number),
                "INTEGER" => Some(Self::Integer),
                "BOOLEAN" => Some(Self::Boolean),
                _ => None,
            }
        }
    }
}
/// `Header` is a representation of OpenAPI v2 specification's Header object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#headerObject>
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Header {
    /// `Description` is a short description of the header.
    #[prost(string, tag = "1")]
    pub description: ::prost::alloc::string::String,
    /// The type of the object. The value MUST be one of "string", "number", "integer", or "boolean". The "array" type is not supported.
    #[prost(string, tag = "2")]
    pub r#type: ::prost::alloc::string::String,
    /// `Format` The extending format for the previously mentioned type.
    #[prost(string, tag = "3")]
    pub format: ::prost::alloc::string::String,
    /// `Default` Declares the value of the header that the server will use if none is provided.
    /// See: <https://tools.ietf.org/html/draft-fge-json-schema-validation-00#section-6.2.>
    /// Unlike JSON Schema this value MUST conform to the defined type for the header.
    #[prost(string, tag = "6")]
    pub default: ::prost::alloc::string::String,
    /// 'Pattern' See <https://tools.ietf.org/html/draft-fge-json-schema-validation-00#section-5.2.3.>
    #[prost(string, tag = "13")]
    pub pattern: ::prost::alloc::string::String,
}
/// `Response` is a representation of OpenAPI v2 specification's Response object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#responseObject>
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    /// `Description` is a short description of the response.
    /// GFM syntax can be used for rich text representation.
    #[prost(string, tag = "1")]
    pub description: ::prost::alloc::string::String,
    /// `Schema` optionally defines the structure of the response.
    /// If `Schema` is not provided, it means there is no content to the response.
    #[prost(message, optional, tag = "2")]
    pub schema: ::core::option::Option<Schema>,
    /// `Headers` A list of headers that are sent with the response.
    /// `Header` name is expected to be a string in the canonical format of the MIME header key
    /// See: <https://golang.org/pkg/net/textproto/#CanonicalMIMEHeaderKey>
    #[prost(map = "string, message", tag = "3")]
    pub headers: ::std::collections::HashMap<::prost::alloc::string::String, Header>,
    /// `Examples` gives per-mimetype response examples.
    /// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#example-object>
    #[prost(map = "string, string", tag = "4")]
    pub examples: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
    /// Custom properties that start with "x-" such as "x-foo" used to describe
    /// extra functionality that is not covered by the standard OpenAPI Specification.
    /// See: <https://swagger.io/docs/specification/2-0/swagger-extensions/>
    #[prost(map = "string, message", tag = "5")]
    pub extensions: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost_types::Value,
    >,
}
/// `Info` is a representation of OpenAPI v2 specification's Info object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#infoObject>
///
/// Example:
///
/// option (grpc.gateway.protoc_gen_openapiv2.options.openapiv2_swagger) = {
/// info: {
/// title: "Echo API";
/// version: "1.0";
/// description: "";
/// contact: {
/// name: "gRPC-Gateway project";
/// url: "<https://github.com/grpc-ecosystem/grpc-gateway";>
/// email: "none@example.com";
/// };
/// license: {
/// name: "BSD 3-Clause License";
/// url: "<https://github.com/grpc-ecosystem/grpc-gateway/blob/main/LICENSE.txt";>
/// };
/// };
/// ...
/// };
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Info {
    /// The title of the application.
    #[prost(string, tag = "1")]
    pub title: ::prost::alloc::string::String,
    /// A short description of the application. GFM syntax can be used for rich
    /// text representation.
    #[prost(string, tag = "2")]
    pub description: ::prost::alloc::string::String,
    /// The Terms of Service for the API.
    #[prost(string, tag = "3")]
    pub terms_of_service: ::prost::alloc::string::String,
    /// The contact information for the exposed API.
    #[prost(message, optional, tag = "4")]
    pub contact: ::core::option::Option<Contact>,
    /// The license information for the exposed API.
    #[prost(message, optional, tag = "5")]
    pub license: ::core::option::Option<License>,
    /// Provides the version of the application API (not to be confused
    /// with the specification version).
    #[prost(string, tag = "6")]
    pub version: ::prost::alloc::string::String,
    /// Custom properties that start with "x-" such as "x-foo" used to describe
    /// extra functionality that is not covered by the standard OpenAPI Specification.
    /// See: <https://swagger.io/docs/specification/2-0/swagger-extensions/>
    #[prost(map = "string, message", tag = "7")]
    pub extensions: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost_types::Value,
    >,
}
/// `Contact` is a representation of OpenAPI v2 specification's Contact object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#contactObject>
///
/// Example:
///
/// option (grpc.gateway.protoc_gen_openapiv2.options.openapiv2_swagger) = {
/// info: {
/// ...
/// contact: {
/// name: "gRPC-Gateway project";
/// url: "<https://github.com/grpc-ecosystem/grpc-gateway";>
/// email: "none@example.com";
/// };
/// ...
/// };
/// ...
/// };
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Contact {
    /// The identifying name of the contact person/organization.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// The URL pointing to the contact information. MUST be in the format of a
    /// URL.
    #[prost(string, tag = "2")]
    pub url: ::prost::alloc::string::String,
    /// The email address of the contact person/organization. MUST be in the format
    /// of an email address.
    #[prost(string, tag = "3")]
    pub email: ::prost::alloc::string::String,
}
/// `License` is a representation of OpenAPI v2 specification's License object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#licenseObject>
///
/// Example:
///
/// option (grpc.gateway.protoc_gen_openapiv2.options.openapiv2_swagger) = {
/// info: {
/// ...
/// license: {
/// name: "BSD 3-Clause License";
/// url: "<https://github.com/grpc-ecosystem/grpc-gateway/blob/main/LICENSE.txt";>
/// };
/// ...
/// };
/// ...
/// };
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct License {
    /// The license name used for the API.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// A URL to the license used for the API. MUST be in the format of a URL.
    #[prost(string, tag = "2")]
    pub url: ::prost::alloc::string::String,
}
/// `ExternalDocumentation` is a representation of OpenAPI v2 specification's
/// ExternalDocumentation object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#externalDocumentationObject>
///
/// Example:
///
/// option (grpc.gateway.protoc_gen_openapiv2.options.openapiv2_swagger) = {
/// ...
/// external_docs: {
/// description: "More about gRPC-Gateway";
/// url: "<https://github.com/grpc-ecosystem/grpc-gateway";>
/// }
/// ...
/// };
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExternalDocumentation {
    /// A short description of the target documentation. GFM syntax can be used for
    /// rich text representation.
    #[prost(string, tag = "1")]
    pub description: ::prost::alloc::string::String,
    /// The URL for the target documentation. Value MUST be in the format
    /// of a URL.
    #[prost(string, tag = "2")]
    pub url: ::prost::alloc::string::String,
}
/// `Schema` is a representation of OpenAPI v2 specification's Schema object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#schemaObject>
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Schema {
    #[prost(message, optional, tag = "1")]
    pub json_schema: ::core::option::Option<JsonSchema>,
    /// Adds support for polymorphism. The discriminator is the schema property
    /// name that is used to differentiate between other schema that inherit this
    /// schema. The property name used MUST be defined at this schema and it MUST
    /// be in the required property list. When used, the value MUST be the name of
    /// this schema or any schema that inherits it.
    #[prost(string, tag = "2")]
    pub discriminator: ::prost::alloc::string::String,
    /// Relevant only for Schema "properties" definitions. Declares the property as
    /// "read only". This means that it MAY be sent as part of a response but MUST
    /// NOT be sent as part of the request. Properties marked as readOnly being
    /// true SHOULD NOT be in the required list of the defined schema. Default
    /// value is false.
    #[prost(bool, tag = "3")]
    pub read_only: bool,
    /// Additional external documentation for this schema.
    #[prost(message, optional, tag = "5")]
    pub external_docs: ::core::option::Option<ExternalDocumentation>,
    /// A free-form property to include an example of an instance for this schema in JSON.
    /// This is copied verbatim to the output.
    #[prost(string, tag = "6")]
    pub example: ::prost::alloc::string::String,
}
/// `JSONSchema` represents properties from JSON Schema taken, and as used, in
/// the OpenAPI v2 spec.
///
/// This includes changes made by OpenAPI v2.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#schemaObject>
///
/// See also: <https://cswr.github.io/JsonSchema/spec/basic_types/,>
/// <https://github.com/json-schema-org/json-schema-spec/blob/master/schema.json>
///
/// Example:
///
/// message SimpleMessage {
/// option (grpc.gateway.protoc_gen_openapiv2.options.openapiv2_schema) = {
/// json_schema: {
/// title: "SimpleMessage"
/// description: "A simple message."
/// required: \\["id"\\]
/// }
/// };
///
/// ```text
/// // Id represents the message identifier.
/// string id = 1; [
///     (grpc.gateway.protoc_gen_openapiv2.options.openapiv2_field) = {
///       description: "The unique identifier of the simple message."
///     }];
/// ```
///
/// }
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct JsonSchema {
    /// Ref is used to define an external reference to include in the message.
    /// This could be a fully qualified proto message reference, and that type must
    /// be imported into the protofile. If no message is identified, the Ref will
    /// be used verbatim in the output.
    /// For example:
    /// `ref: ".google.protobuf.Timestamp"`.
    #[prost(string, tag = "3")]
    pub r#ref: ::prost::alloc::string::String,
    /// The title of the schema.
    #[prost(string, tag = "5")]
    pub title: ::prost::alloc::string::String,
    /// A short description of the schema.
    #[prost(string, tag = "6")]
    pub description: ::prost::alloc::string::String,
    #[prost(string, tag = "7")]
    pub default: ::prost::alloc::string::String,
    #[prost(bool, tag = "8")]
    pub read_only: bool,
    /// A free-form property to include a JSON example of this field. This is copied
    /// verbatim to the output swagger.json. Quotes must be escaped.
    /// This property is the same for 2.0 and 3.0.0 <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/3.0.0.md#schemaObject>  <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#schemaObject>
    #[prost(string, tag = "9")]
    pub example: ::prost::alloc::string::String,
    #[prost(double, tag = "10")]
    pub multiple_of: f64,
    /// Maximum represents an inclusive upper limit for a numeric instance. The
    /// value of MUST be a number,
    #[prost(double, tag = "11")]
    pub maximum: f64,
    #[prost(bool, tag = "12")]
    pub exclusive_maximum: bool,
    /// minimum represents an inclusive lower limit for a numeric instance. The
    /// value of MUST be a number,
    #[prost(double, tag = "13")]
    pub minimum: f64,
    #[prost(bool, tag = "14")]
    pub exclusive_minimum: bool,
    #[prost(uint64, tag = "15")]
    pub max_length: u64,
    #[prost(uint64, tag = "16")]
    pub min_length: u64,
    #[prost(string, tag = "17")]
    pub pattern: ::prost::alloc::string::String,
    #[prost(uint64, tag = "20")]
    pub max_items: u64,
    #[prost(uint64, tag = "21")]
    pub min_items: u64,
    #[prost(bool, tag = "22")]
    pub unique_items: bool,
    #[prost(uint64, tag = "24")]
    pub max_properties: u64,
    #[prost(uint64, tag = "25")]
    pub min_properties: u64,
    #[prost(string, repeated, tag = "26")]
    pub required: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// Items in 'array' must be unique.
    #[prost(string, repeated, tag = "34")]
    pub array: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(enumeration = "json_schema::JsonSchemaSimpleTypes", repeated, tag = "35")]
    pub r#type: ::prost::alloc::vec::Vec<i32>,
    /// `Format`
    #[prost(string, tag = "36")]
    pub format: ::prost::alloc::string::String,
    /// Items in `enum` must be unique <https://tools.ietf.org/html/draft-fge-json-schema-validation-00#section-5.5.1>
    #[prost(string, repeated, tag = "46")]
    pub r#enum: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// Additional field level properties used when generating the OpenAPI v2 file.
    #[prost(message, optional, tag = "1001")]
    pub field_configuration: ::core::option::Option<json_schema::FieldConfiguration>,
    /// Custom properties that start with "x-" such as "x-foo" used to describe
    /// extra functionality that is not covered by the standard OpenAPI Specification.
    /// See: <https://swagger.io/docs/specification/2-0/swagger-extensions/>
    #[prost(map = "string, message", tag = "48")]
    pub extensions: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost_types::Value,
    >,
}
/// Nested message and enum types in `JSONSchema`.
pub mod json_schema {
    /// 'FieldConfiguration' provides additional field level properties used when generating the OpenAPI v2 file.
    /// These properties are not defined by OpenAPIv2, but they are used to control the generation.
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct FieldConfiguration {
        /// Alternative parameter name when used as path parameter. If set, this will
        /// be used as the complete parameter name when this field is used as a path
        /// parameter. Use this to avoid having auto generated path parameter names
        /// for overlapping paths.
        #[prost(string, tag = "47")]
        pub path_param_name: ::prost::alloc::string::String,
    }
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum JsonSchemaSimpleTypes {
        Unknown = 0,
        Array = 1,
        Boolean = 2,
        Integer = 3,
        Null = 4,
        Number = 5,
        Object = 6,
        String = 7,
    }
    impl JsonSchemaSimpleTypes {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                JsonSchemaSimpleTypes::Unknown => "UNKNOWN",
                JsonSchemaSimpleTypes::Array => "ARRAY",
                JsonSchemaSimpleTypes::Boolean => "BOOLEAN",
                JsonSchemaSimpleTypes::Integer => "INTEGER",
                JsonSchemaSimpleTypes::Null => "NULL",
                JsonSchemaSimpleTypes::Number => "NUMBER",
                JsonSchemaSimpleTypes::Object => "OBJECT",
                JsonSchemaSimpleTypes::String => "STRING",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "UNKNOWN" => Some(Self::Unknown),
                "ARRAY" => Some(Self::Array),
                "BOOLEAN" => Some(Self::Boolean),
                "INTEGER" => Some(Self::Integer),
                "NULL" => Some(Self::Null),
                "NUMBER" => Some(Self::Number),
                "OBJECT" => Some(Self::Object),
                "STRING" => Some(Self::String),
                _ => None,
            }
        }
    }
}
/// `Tag` is a representation of OpenAPI v2 specification's Tag object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#tagObject>
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Tag {
    /// The name of the tag. Use it to allow override of the name of a
    /// global Tag object, then use that name to reference the tag throughout the
    /// OpenAPI file.
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    /// A short description for the tag. GFM syntax can be used for rich text
    /// representation.
    #[prost(string, tag = "2")]
    pub description: ::prost::alloc::string::String,
    /// Additional external documentation for this tag.
    #[prost(message, optional, tag = "3")]
    pub external_docs: ::core::option::Option<ExternalDocumentation>,
    /// Custom properties that start with "x-" such as "x-foo" used to describe
    /// extra functionality that is not covered by the standard OpenAPI Specification.
    /// See: <https://swagger.io/docs/specification/2-0/swagger-extensions/>
    #[prost(map = "string, message", tag = "4")]
    pub extensions: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost_types::Value,
    >,
}
/// `SecurityDefinitions` is a representation of OpenAPI v2 specification's
/// Security Definitions object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#securityDefinitionsObject>
///
/// A declaration of the security schemes available to be used in the
/// specification. This does not enforce the security schemes on the operations
/// and only serves to provide the relevant details for each scheme.
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SecurityDefinitions {
    /// A single security scheme definition, mapping a "name" to the scheme it
    /// defines.
    #[prost(map = "string, message", tag = "1")]
    pub security: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        SecurityScheme,
    >,
}
/// `SecurityScheme` is a representation of OpenAPI v2 specification's
/// Security Scheme object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#securitySchemeObject>
///
/// Allows the definition of a security scheme that can be used by the
/// operations. Supported schemes are basic authentication, an API key (either as
/// a header or as a query parameter) and OAuth2's common flows (implicit,
/// password, application and access code).
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SecurityScheme {
    /// The type of the security scheme. Valid values are "basic",
    /// "apiKey" or "oauth2".
    #[prost(enumeration = "security_scheme::Type", tag = "1")]
    pub r#type: i32,
    /// A short description for security scheme.
    #[prost(string, tag = "2")]
    pub description: ::prost::alloc::string::String,
    /// The name of the header or query parameter to be used.
    /// Valid for apiKey.
    #[prost(string, tag = "3")]
    pub name: ::prost::alloc::string::String,
    /// The location of the API key. Valid values are "query" or
    /// "header".
    /// Valid for apiKey.
    #[prost(enumeration = "security_scheme::In", tag = "4")]
    pub r#in: i32,
    /// The flow used by the OAuth2 security scheme. Valid values are
    /// "implicit", "password", "application" or "accessCode".
    /// Valid for oauth2.
    #[prost(enumeration = "security_scheme::Flow", tag = "5")]
    pub flow: i32,
    /// The authorization URL to be used for this flow. This SHOULD be in
    /// the form of a URL.
    /// Valid for oauth2/implicit and oauth2/accessCode.
    #[prost(string, tag = "6")]
    pub authorization_url: ::prost::alloc::string::String,
    /// The token URL to be used for this flow. This SHOULD be in the
    /// form of a URL.
    /// Valid for oauth2/password, oauth2/application and oauth2/accessCode.
    #[prost(string, tag = "7")]
    pub token_url: ::prost::alloc::string::String,
    /// The available scopes for the OAuth2 security scheme.
    /// Valid for oauth2.
    #[prost(message, optional, tag = "8")]
    pub scopes: ::core::option::Option<Scopes>,
    /// Custom properties that start with "x-" such as "x-foo" used to describe
    /// extra functionality that is not covered by the standard OpenAPI Specification.
    /// See: <https://swagger.io/docs/specification/2-0/swagger-extensions/>
    #[prost(map = "string, message", tag = "9")]
    pub extensions: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost_types::Value,
    >,
}
/// Nested message and enum types in `SecurityScheme`.
pub mod security_scheme {
    /// The type of the security scheme. Valid values are "basic",
    /// "apiKey" or "oauth2".
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Type {
        Invalid = 0,
        Basic = 1,
        ApiKey = 2,
        Oauth2 = 3,
    }
    impl Type {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Type::Invalid => "TYPE_INVALID",
                Type::Basic => "TYPE_BASIC",
                Type::ApiKey => "TYPE_API_KEY",
                Type::Oauth2 => "TYPE_OAUTH2",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "TYPE_INVALID" => Some(Self::Invalid),
                "TYPE_BASIC" => Some(Self::Basic),
                "TYPE_API_KEY" => Some(Self::ApiKey),
                "TYPE_OAUTH2" => Some(Self::Oauth2),
                _ => None,
            }
        }
    }
    /// The location of the API key. Valid values are "query" or "header".
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum In {
        Invalid = 0,
        Query = 1,
        Header = 2,
    }
    impl In {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                In::Invalid => "IN_INVALID",
                In::Query => "IN_QUERY",
                In::Header => "IN_HEADER",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "IN_INVALID" => Some(Self::Invalid),
                "IN_QUERY" => Some(Self::Query),
                "IN_HEADER" => Some(Self::Header),
                _ => None,
            }
        }
    }
    /// The flow used by the OAuth2 security scheme. Valid values are
    /// "implicit", "password", "application" or "accessCode".
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum Flow {
        Invalid = 0,
        Implicit = 1,
        Password = 2,
        Application = 3,
        AccessCode = 4,
    }
    impl Flow {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Flow::Invalid => "FLOW_INVALID",
                Flow::Implicit => "FLOW_IMPLICIT",
                Flow::Password => "FLOW_PASSWORD",
                Flow::Application => "FLOW_APPLICATION",
                Flow::AccessCode => "FLOW_ACCESS_CODE",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "FLOW_INVALID" => Some(Self::Invalid),
                "FLOW_IMPLICIT" => Some(Self::Implicit),
                "FLOW_PASSWORD" => Some(Self::Password),
                "FLOW_APPLICATION" => Some(Self::Application),
                "FLOW_ACCESS_CODE" => Some(Self::AccessCode),
                _ => None,
            }
        }
    }
}
/// `SecurityRequirement` is a representation of OpenAPI v2 specification's
/// Security Requirement object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#securityRequirementObject>
///
/// Lists the required security schemes to execute this operation. The object can
/// have multiple security schemes declared in it which are all required (that
/// is, there is a logical AND between the schemes).
///
/// The name used for each property MUST correspond to a security scheme
/// declared in the Security Definitions.
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SecurityRequirement {
    /// Each name must correspond to a security scheme which is declared in
    /// the Security Definitions. If the security scheme is of type "oauth2",
    /// then the value is a list of scope names required for the execution.
    /// For other security scheme types, the array MUST be empty.
    #[prost(map = "string, message", tag = "1")]
    pub security_requirement: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        security_requirement::SecurityRequirementValue,
    >,
}
/// Nested message and enum types in `SecurityRequirement`.
pub mod security_requirement {
    /// If the security scheme is of type "oauth2", then the value is a list of
    /// scope names required for the execution. For other security scheme types,
    /// the array MUST be empty.
    #[serde_with::serde_as]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SecurityRequirementValue {
        #[prost(string, repeated, tag = "1")]
        pub scope: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    }
}
/// `Scopes` is a representation of OpenAPI v2 specification's Scopes object.
///
/// See: <https://github.com/OAI/OpenAPI-Specification/blob/3.0.0/versions/2.0.md#scopesObject>
///
/// Lists the available scopes for an OAuth2 security scheme.
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Scopes {
    /// Maps between a name of a scope to a short description of it (as the value
    /// of the property).
    #[prost(map = "string, string", tag = "1")]
    pub scope: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
}
/// Scheme describes the schemes supported by the OpenAPI Swagger
/// and Operation objects.
#[serde_with::serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Scheme {
    Unknown = 0,
    Http = 1,
    Https = 2,
    Ws = 3,
    Wss = 4,
}
impl Scheme {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Scheme::Unknown => "UNKNOWN",
            Scheme::Http => "HTTP",
            Scheme::Https => "HTTPS",
            Scheme::Ws => "WS",
            Scheme::Wss => "WSS",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "UNKNOWN" => Some(Self::Unknown),
            "HTTP" => Some(Self::Http),
            "HTTPS" => Some(Self::Https),
            "WS" => Some(Self::Ws),
            "WSS" => Some(Self::Wss),
            _ => None,
        }
    }
}
