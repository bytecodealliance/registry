use prost::Message;

fn main() -> anyhow::Result<()> {
    let proto_files = &["warg/protocol/warg.proto", "warg/transparency/proofs.proto"];

    // Tell cargo to recompile if any of these proto files are changed
    for proto_file in proto_files {
        println!("cargo:rerun-if-changed={proto_file}");
    }

    let file_descriptor_set = protox::Compiler::new(["."])?
        .include_source_info(true)
        .include_imports(true)
        .open_files(proto_files)?
        .file_descriptor_set();

    let file_descriptor_set_bytes = file_descriptor_set.encode_to_vec();

    prost_build::Config::new()
        // Override prost-types with pbjson-types
        .compile_well_known_types()
        .extern_path(".google.protobuf", "::pbjson_types")
        .compile_fds(file_descriptor_set)?;

    pbjson_build::Builder::new()
        .register_descriptors(&file_descriptor_set_bytes)?
        .build(&[".warg.protocol", ".warg.transparency"])?;

    Ok(())
}
