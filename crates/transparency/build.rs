use regex::Regex;
use std::{env, io::Result, path::PathBuf, process::Command};

fn main() -> Result<()> {
    verify_protoc_version(15, 0);
    let proofs_proto = PathBuf::from("proto/warg/transparency/proofs.proto");
    let proto_files = vec![proofs_proto];
    let root = PathBuf::from("proto");

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
        .build(&[".warg.transparency"])?;

    Ok(())
}

// NB: protoc changed versioning schemes with v3.20 (removed major version), see https://protobuf.dev/support/version-support/
// This means that after 3.20.x came 21.x - the major and minor version arguments here refer to the new scheme and would have previously been minor and patch versions respectively.
fn verify_protoc_version(min_major: u32, min_minor: u32) {
    let protoc_path = prost_build::protoc_from_env();
    let protoc_version_output = Command::new(protoc_path)
        .args(["--version"])
        .output()
        .unwrap();
    let protoc_version = String::from_utf8(protoc_version_output.stdout).unwrap();

    // based on semver.org's recommended regex, modified for lib name and optional patch capture group to accomodate libprotoc's versioning scheme
    let re = Regex::new(r"^libprotoc (?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.?(?P<patch>0|[1-9]\d*)?(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<buildmetadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$").unwrap();
    let caps = re.captures(protoc_version.trim()).unwrap();
    let major = caps.name("major").unwrap().as_str().parse::<u32>().unwrap();
    let minor = caps.name("minor").unwrap().as_str().parse::<u32>().unwrap();
    let version_match = if let Some(patch) = caps.name("patch") {
        let patch = patch.as_str().parse::<u32>().unwrap();
        major > 3 || (major == 3 && minor > min_major && patch > min_minor)
    } else {
        major > min_major || (major == min_major && minor > min_minor)
    };

    if !version_match {
        panic!(
                "Building this crate requires a version of protoc (libprotoc) >={min_major}.{min_minor}, found: {protoc_version}\nPlease install a suitable version of protobuf (see https://github.com/protocolbuffers/protobuf for downloads and instructions as well as https://protobuf.dev/support/version-support/ for info on protobuf's versioning schema)"
            )
    }
}
