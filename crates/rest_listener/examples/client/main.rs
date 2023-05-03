#![allow(missing_docs, unused_variables, trivial_casts)]


#[allow(unused_imports)]
use futures::{future, Stream, stream};
#[allow(unused_imports)]
use openapi_client::{Api, ApiNoContext, Client, ContextWrapperExt, models,
                      WargFetchLogsResponse,
                      WargGetPackageResponse,
                      WargGetPackageRecordResponse,
                      WargPublishPackageResponse,
                      WargFetchCheckpointResponse,
                      WargProveConsistencyResponse,
                      WargProveInclusionResponse,
                     };
use clap::{App, Arg};

#[allow(unused_imports)]
use log::info;

// swagger::Has may be unused if there are no examples
#[allow(unused_imports)]
use swagger::{AuthData, ContextBuilder, EmptyContext, Has, Push, XSpanIdString};

type ClientContext = swagger::make_context_ty!(ContextBuilder, EmptyContext, Option<AuthData>, XSpanIdString);

// rt may be unused if there are no examples
#[allow(unused_mut)]
fn main() {
    env_logger::init();

    let matches = App::new("client")
        .arg(Arg::with_name("operation")
            .help("Sets the operation to run")
            .possible_values(&[
                "WargFetchLogs",
                "WargGetPackage",
                "WargGetPackageRecord",
                "WargPublishPackage",
                "WargFetchCheckpoint",
                "WargProveConsistency",
                "WargProveInclusion",
            ])
            .required(true)
            .index(1))
        .arg(Arg::with_name("https")
            .long("https")
            .help("Whether to use HTTPS or not"))
        .arg(Arg::with_name("host")
            .long("host")
            .takes_value(true)
            .default_value("localhost")
            .help("Hostname to contact"))
        .arg(Arg::with_name("port")
            .long("port")
            .takes_value(true)
            .default_value("8080")
            .help("Port to contact"))
        .get_matches();

    let is_https = matches.is_present("https");
    let base_url = format!("{}://{}:{}",
                           if is_https { "https" } else { "http" },
                           matches.value_of("host").unwrap(),
                           matches.value_of("port").unwrap());

    let context: ClientContext =
        swagger::make_context!(ContextBuilder, EmptyContext, None as Option<AuthData>, XSpanIdString::default());

    let mut client : Box<dyn ApiNoContext<ClientContext>> = if matches.is_present("https") {
        // Using Simple HTTPS
        let client = Box::new(Client::try_new_https(&base_url)
            .expect("Failed to create HTTPS client"));
        Box::new(client.with_context(context))
    } else {
        // Using HTTP
        let client = Box::new(Client::try_new_http(
            &base_url)
            .expect("Failed to create HTTP client"));
        Box::new(client.with_context(context))
    };

    let mut rt = tokio::runtime::Runtime::new().unwrap();

    match matches.value_of("operation") {
        Some("WargFetchLogs") => {
            let result = rt.block_on(client.warg_fetch_logs(
                  Some("root_period_algo_example".to_string()),
                  Some(swagger::ByteArray(Vec::from("BYTE_ARRAY_DATA_HERE"))),
                  Some("operator_period_algo_example".to_string()),
                  Some(swagger::ByteArray(Vec::from("BYTE_ARRAY_DATA_HERE")))
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        Some("WargGetPackage") => {
            let result = rt.block_on(client.warg_get_package(
                  "package_id_example".to_string()
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        Some("WargGetPackageRecord") => {
            let result = rt.block_on(client.warg_get_package_record(
                  "package_id_example".to_string(),
                  "record_id_example".to_string()
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        Some("WargPublishPackage") => {
            let result = rt.block_on(client.warg_publish_package(
                  Some("name_example".to_string()),
                  Some(swagger::ByteArray(Vec::from("BYTE_ARRAY_DATA_HERE"))),
                  Some("record_period_key_id_example".to_string()),
                  Some("record_period_signature_example".to_string())
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        Some("WargFetchCheckpoint") => {
            let result = rt.block_on(client.warg_fetch_checkpoint(
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        Some("WargProveConsistency") => {
            let result = rt.block_on(client.warg_prove_consistency(
                  Some("old_root_period_algo_example".to_string()),
                  Some(swagger::ByteArray(Vec::from("BYTE_ARRAY_DATA_HERE"))),
                  Some("new_root_period_algo_example".to_string()),
                  Some(swagger::ByteArray(Vec::from("BYTE_ARRAY_DATA_HERE")))
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        Some("WargProveInclusion") => {
            let result = rt.block_on(client.warg_prove_inclusion(
                  Some("checkpoint_period_log_root_period_algo_example".to_string()),
                  Some(swagger::ByteArray(Vec::from("BYTE_ARRAY_DATA_HERE"))),
                  Some(789),
                  Some("checkpoint_period_map_root_period_algo_example".to_string()),
                  Some(swagger::ByteArray(Vec::from("BYTE_ARRAY_DATA_HERE")))
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        _ => {
            panic!("Invalid operation provided")
        }
    }
}
