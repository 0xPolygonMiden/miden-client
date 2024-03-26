use std::{fs::File, io::Write, path::PathBuf};

const DEFAULT_CONFIG: &str = include_str!("../../miden-client.toml");

pub(crate) fn initialize_client(config_file_path: PathBuf) -> Result<(), String> {
    println!("Creating config file at: {:?}", config_file_path);
    let mut file_handle = File::options()
        .write(true)
        .create_new(true)
        .open(config_file_path)
        .map_err(|err| format!("error opening the file: {err}"))?;
    file_handle
        .write(DEFAULT_CONFIG.as_bytes())
        .map_err(|err| format!("error writing to file: {err}"))?;

    Ok(())
}
