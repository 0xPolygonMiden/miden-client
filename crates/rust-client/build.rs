use std::{
    env, fs,
    path::{Path, PathBuf},
};

use miden_rpc_proto::write_proto;
use miette::IntoDiagnostic;
use prost::Message;

const TONIC_CLIENT_PROTO_OUT_DIR: &str = "src/rpc/tonic_client/generated";
const WEB_TONIC_CLIENT_PROTO_OUT_DIR: &str = "src/rpc/web_tonic_client/generated";

fn main() -> miette::Result<()> {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR should be set");
    let dest_path = PathBuf::from(out_dir);

    write_proto(&dest_path).unwrap();
    compile_tonic_client_proto(&dest_path)
}

// Compiles the protobuf files into a file descriptor used to generate Rust types
fn compile_tonic_client_proto(proto_dir: &Path) -> miette::Result<()> {
    // Compute the compiler's target file path.
    let out = env::var("OUT_DIR").into_diagnostic()?;
    let file_descriptor_path = PathBuf::from(out).join("file_descriptor_set.bin");

    // Compile the proto file
    let protos = &[proto_dir.join("rpc.proto")];
    let includes = &[proto_dir];
    let file_descriptors = protox::compile(protos, includes)?;
    fs::write(&file_descriptor_path, file_descriptors.encode_to_vec()).into_diagnostic()?;

    let mut prost_config = prost_build::Config::new();
    prost_config.skip_debug(["AccountId", "Digest"]);

    let mut web_tonic_prost_config = prost_build::Config::new();
    web_tonic_prost_config.skip_debug(["AccountId", "Digest"]);

    // Generate the stub of the user facing server from its proto file
    tonic_build::configure()
        .build_transport(false)
        .build_server(false)
        .file_descriptor_set_path(&file_descriptor_path)
        .skip_protoc_run()
        .out_dir(WEB_TONIC_CLIENT_PROTO_OUT_DIR)
        .compile_with_config(web_tonic_prost_config, protos, includes)
        .into_diagnostic()?;

    tonic_build::configure()
        .build_server(false)
        .file_descriptor_set_path(&file_descriptor_path)
        .skip_protoc_run()
        .out_dir(TONIC_CLIENT_PROTO_OUT_DIR)
        .compile_with_config(prost_config, protos, includes)
        .into_diagnostic()?;

    Ok(())
}
