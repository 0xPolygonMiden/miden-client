use std::{
    env, fs,
    io::Write,
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

    // updates the generated files from protobuf. Only do so when this is not docs.rs building the
    // documentation.
    if env::var("DOCS_RS").is_err() {
        write_proto(&dest_path).unwrap();
        compile_tonic_client_proto(&dest_path)?;
        replace_no_std_types();
    }

    Ok(())
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

/// This function replaces all "std::result" with "core::result" in the generated "rpc.rs" file
/// for the web tonic client. This is needed as `tonic_build` doesn't generate `no_std` compatible
/// files and we want to build wasm without `std`.
fn replace_no_std_types() {
    let path = WEB_TONIC_CLIENT_PROTO_OUT_DIR.to_string() + "/rpc.rs";
    let file_str = fs::read_to_string(&path).unwrap();
    let new_file_str = file_str
        .replace("std::result", "core::result")
        .replace("std::marker", "core::marker");

    let mut f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    f.write_all(new_file_str.as_bytes()).unwrap();
}
