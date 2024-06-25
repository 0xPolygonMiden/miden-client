use std::{
    env, fs,
    path::{Path, PathBuf},
};

use miden_rpc_proto::write_proto;
use miette::IntoDiagnostic;
use prost::Message;

fn main() -> miette::Result<()> {
    let proto_dir = Path::new("./src/client/rpc/proto");
    if !proto_dir.exists() {
        fs::create_dir(proto_dir).into_diagnostic()?;
    }

    write_proto(proto_dir).unwrap();

    compile_types()
}

/// Compiles the protobuf files into a file descriptor used to generate Rust types
fn compile_types() -> miette::Result<()> {
    // Compute the directory of the `proto` definitions
    let cwd: PathBuf = env::current_dir().into_diagnostic()?;

    let proto_dir: PathBuf = cwd.join("src/client/rpc/tonic_client/proto");

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
        .out_dir("src/client/rpc/tonic_client/generated")
        .compile_with_config(prost_config, protos, includes)
        .into_diagnostic()?;

    Ok(())
}
