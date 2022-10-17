use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &["src/operator/operator.proto", "src/package/package.proto"],
        &["src/"],
    )?;
    Ok(())
}
