use std::io::Result;

use proto_scan_build::CompileScan as _;

fn main() -> Result<()> {
    let mut config = prost_build::Config::new();
    config.protoc_arg("--experimental_allow_proto3_optional");
    config.compile_scan(&["src/proto/testing.proto"], &["src/"])?;

    let prost_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("prost");
    std::fs::create_dir_all(&prost_dir)?;
    config.out_dir(prost_dir);
    config.type_attribute(".", "#[derive(::proto_scan::ScanMessage)]");
    config.compile_protos(&["src/proto/testing.proto"], &["src/"])?;
    Ok(())
}
