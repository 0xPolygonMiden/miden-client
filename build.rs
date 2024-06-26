use std::env;
use std::fs;
use std::path::Path;
use std::io;

fn main() -> io::Result<()> {
    // Log that the build script is running
    println!("Running build.rs script...");

    // Check if the "wasm" feature is enabled
    let is_wasm = env::var("CARGO_FEATURE_WASM").is_ok();

    // Set the crate type based on the enabled feature
    let crate_type = if is_wasm {
        "cdylib"
    } else {
        "lib"
    };

    // Read the existing Cargo.toml
    let cargo_toml_path = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
    let cargo_toml_content = fs::read_to_string(&cargo_toml_path).expect("Unable to read Cargo.toml");

    // Modify the crate-type line
    let mut new_cargo_toml_content = String::new();
    for line in cargo_toml_content.lines() {
        if line.starts_with("crate-type =") {
            new_cargo_toml_content.push_str(&format!(r#"crate-type = ["{}"]"#, crate_type));
        } else {
            new_cargo_toml_content.push_str(line);
        }
        new_cargo_toml_content.push('\n');
    }

    // Write the modified Cargo.toml to the output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("Cargo.toml");
    fs::write(&dest_path, new_cargo_toml_content).expect("Unable to write new Cargo.toml");

    // Execute additional functionality only if the "wasm" feature is enabled
    if is_wasm {
        use tonic_build;

        tonic_build::configure().build_server(false).build_client(true).compile(
            &[
                "src/web_client/rpc/proto/account.proto",
                "src/web_client/rpc/proto/block_header.proto",
                "src/web_client/rpc/proto/block_producer.proto",
                "src/web_client/rpc/proto/digest.proto",
                "src/web_client/rpc/proto/merkle.proto",
                "src/web_client/rpc/proto/mmr.proto",
                "src/web_client/rpc/proto/note.proto",
                "src/web_client/rpc/proto/requests.proto",
                "src/web_client/rpc/proto/responses.proto",
                "src/web_client/rpc/proto/rpc.proto",
                "src/web_client/rpc/proto/smt.proto",
                "src/web_client/rpc/proto/store.proto",
                "src/web_client/rpc/proto/transaction.proto",
            ],
            &["src/web_client/rpc/proto"],
        )?;
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", cargo_toml_path.to_str().unwrap());
    Ok(())
}