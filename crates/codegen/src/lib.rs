use std::{error::Error, fs::read_dir};

use poem_grpc_build::Config;

pub fn build() -> Result<(), Box<dyn Error>> {
    let mut protos = Vec::new();

    for item in read_dir("./proto")? {
        let entry = item?;
        if !entry.metadata()?.is_file() {
            continue;
        }
        let path = entry.path();

        if let Some("proto") = path.extension().and_then(|ext| ext.to_str()) {
            protos.push(path);
        }
    }

    Config::new()
        .file_descriptor_set_path("file_descriptor_set.bin")
        .codec("poem_grpc::codec::JsonI64ToStringCodec")
        .type_attribute(".", "#[derive(serde::Deserialize, serde::Serialize)]")
        .client_middleware("gear_microkit::middlewares::AddClientHeaders")
        .client_middleware("gear_microkit::middlewares::ClientTracing")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&protos, &["./proto"])?;
    Ok(())
}
