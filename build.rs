use std::{
    env, fs,
    path::{Path, PathBuf},
};

use miden_rpc_proto::write_proto;
use miette::IntoDiagnostic;
use prost::Message;
use tempfile::TempDir;

const OUT_DIR: &str = "src/client/rpc/tonic_client/generated";

fn main() -> miette::Result<()> {
    let temp_proto_dir = TempDir::new().into_diagnostic()?;

    write_proto(temp_proto_dir.path()).unwrap();
    compile_types(temp_proto_dir.path())
}

/// Compiles the protobuf files into a file descriptor used to generate Rust types
fn compile_types(proto_dir: &Path) -> miette::Result<()> {
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

    // Generate the stub of the user facing server from its proto file
    tonic_build::configure()
        .file_descriptor_set_path(&file_descriptor_path)
        .skip_protoc_run()
        .out_dir(OUT_DIR)
        .compile_with_config(prost_config, protos, includes)
        .into_diagnostic()?;

    Ok(())
}
