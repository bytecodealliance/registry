use std::{env, io::Result, path::PathBuf, process::Command};
use regex::Regex;

fn main() -> Result<()> {
    check_protoc_version()?;
    let warg_proto = PathBuf::from("../../proto/warg/protocol/warg.proto");
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
        .build(&[".warg.protocol"])?;

    Ok(())
}

fn check_protoc_version() -> Result<()> {
    let protoc_path = prost_build::protoc_from_env();
    let protoc_version_output = Command::new(protoc_path).args(["--version"]).output()?;
    let protoc_version = String::from_utf8(protoc_version_output.stdout)
        .unwrap();
    // semver.org recommended regex, modified for optional patch capture group to accomodate libprotoc's versioning scheme and lib name
    let re = Regex::new(r"^libprotoc (?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.?(?P<patch>0|[1-9]\d*)?(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<buildmetadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$").unwrap();
    let caps = re.captures(protoc_version.trim()).unwrap();
    let major = caps.name("major").unwrap().as_str().parse::<u32>().unwrap();
    let minor = caps.name("minor").unwrap().as_str().parse::<u32>().unwrap();

    if major < 3 || (major == 3 && minor < 15) {
        panic!(
            "Building this crate requires a version of protoc (libprotoc) >=3.15, found: {}",
            protoc_version.trim()
        )
    }
    Ok(())
}

