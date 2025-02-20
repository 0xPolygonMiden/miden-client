use std::{
    env, fs,
    path::{Path, PathBuf},
};

use miden_client::account::component::{
    AccountComponentMetadata, AccountComponentTemplate, COMPONENT_TEMPLATE_EXTENSION,
};
use miden_lib::{
    account::components::{basic_fungible_faucet_library, basic_wallet_library},
    utils::Serializable,
};
use miden_objects::assembly::Library;

fn main() {
    build_component_template(
        &PathBuf::from("templates/basic-fungible-faucet.toml"),
        basic_fungible_faucet_library(),
    );

    build_component_template(&PathBuf::from("templates/basic-auth.toml"), basic_wallet_library());
}

/// Builds a component template and stores it under `{OUT_DIR}/templates`.
pub fn build_component_template(metadata_path: &Path, library: Library) {
    let toml_string = fs::read_to_string(metadata_path).expect("failed to read file");

    let template_metadata =
        AccountComponentMetadata::from_toml(&toml_string).expect("faucet toml is well-formed");

    let faucet_component_template =
        AccountComponentTemplate::new(template_metadata, library).to_bytes();

    // Write the file
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR environment variable not set");
    let templates_out_dir = PathBuf::from(out_dir).join("templates");
    fs::create_dir_all(&templates_out_dir)
        .expect("Failed to create templates directory in OUT_DIR");

    let mut output_filename = metadata_path
        .file_stem()
        .expect("metadata path should have a file stem")
        .to_os_string();
    output_filename.push(format!(".{COMPONENT_TEMPLATE_EXTENSION}"));

    let output_file = templates_out_dir.join(output_filename);
    fs::write(&output_file, &faucet_component_template)
        .expect("Failed to write faucet component template file");
}
