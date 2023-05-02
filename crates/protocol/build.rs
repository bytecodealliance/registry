use std::{env, io::Result, path::PathBuf};

fn main() -> Result<()> {
    let warg_proto = PathBuf::from("../../proto/warg/protocol/v1/warg.proto");
    let proto_files = vec![warg_proto];
    let root = PathBuf::from("../../proto");

    // Tell cargo to recompile if any of these proto files are changed
    for proto_file in &proto_files {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }

    let descriptor_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("proto_descriptor.bin");

    prost_build::Config::new()
        // Save descriptors to file
        .file_descriptor_set_path(&descriptor_path)
        // Override prost-types with pbjson-types
        .compile_well_known_types()
        .extern_path(".google.protobuf", "::pbjson_types")
        // Generate prost structs
        .compile_protos(&proto_files, &[root])?;

    let descriptor_set = std::fs::read(descriptor_path)?;
    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)?
        .build(&[".warg.protocol.v1"])?;

    // NOTE: Multiple compiles to work around tonic_build issue.
    // SEE: https://github.com/hyperium/tonic/issues/259

    #[cfg(feature = "grpc")]
    tonic_build::configure()
        .emit_rerun_if_changed(true)
        .extern_path(".google.protobuf.Any", "::prost_wkt_types::Any")
        .extern_path(".google.protobuf.Duration", "::prost_wkt_types::Duration")
        .extern_path(".google.protobuf.Timestamp", "::prost_wkt_types::Timestamp")
        .type_attribute(".", "#[serde_with::serde_as]")
        .type_attribute(".", "#[derive(serde::Serialize,serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir("src/gen")
        .extern_path(".google.api", "crate::google_pb")
        .extern_path(".google.rpc", "crate::google_pb")
        .compile(
            &[
                "../../proto/warg/protocol/v1/service.proto",
                "../../proto/warg/protocol/v1/warg.proto",
            ],
            &["../../proto"],
        )?;

    #[cfg(feature = "grpc")]
    tonic_build::configure()
        .emit_rerun_if_changed(true)
        .extern_path(".google.protobuf.Any", "::prost_wkt_types::Any")
        .extern_path(".google.protobuf.Duration", "::prost_wkt_types::Duration")
        .extern_path(".google.protobuf.Timestamp", "::prost_wkt_types::Timestamp")
        .type_attribute(".", "#[serde_with::serde_as]")
        .type_attribute(".", "#[derive(serde::Serialize,serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir("src/gen")
        .compile(
            &[
                "../../proto/google/api/annotations.proto",
                "../../proto/google/api/http.proto",
                "../../proto/google/rpc/http.proto",
            ],
            &["../../proto"],
        )?;

    Ok(())
}
