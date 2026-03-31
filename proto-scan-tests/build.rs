use std::io::Result;

use proto_scan_build::Config;

fn main() -> Result<()> {
    let protos = [
        "src/proto/map.proto",
        "src/proto/oneof.proto",
        "src/proto/optional.proto",
        "src/proto/testing.proto",
        "src/proto/empty_message.proto",
        "src/proto/groups.proto",
    ];
    let make_config = || {
        let mut config = prost_build::Config::new();
        config.protoc_arg("--experimental_allow_proto3_optional");
        config
    };
    Config::from(make_config()).compile_protos(&protos, &["src/"])?;

    for f in protos {
        println!("cargo:rerun-if-changed={f}");
    }

    let prost_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("prost");
    std::fs::create_dir_all(&prost_dir)?;
    let mut config = make_config();
    config.out_dir(prost_dir);
    config.type_attribute(".", "#[derive(::proto_scan::ScanMessage)]");
    config.compile_protos(&protos, &["src/"])?;
    Ok(())
}
