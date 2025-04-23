use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use figment::{
    Figment,
    providers::{Format, Toml},
};
use miden_client::{Client, account::AccountId};
use tracing::info;

use super::{CLIENT_CONFIG_FILE_NAME, config::CliConfig, get_account_with_id_prefix};
use crate::{errors::CliError, faucet_details_map::FaucetDetailsMap};

pub(crate) const SHARED_TOKEN_DOCUMENTATION: &str = "There are two accepted formats for the asset:
- `<AMOUNT>::<FAUCET_ID>` where `<AMOUNT>` is in the faucet base units.
- `<AMOUNT>::<TOKEN_SYMBOL>` where `<AMOUNT>` is a decimal number representing the quantity of
the token (specified to the precision allowed by the token's decimals), and `<TOKEN_SYMBOL>`
is a symbol tracked in the token symbol map file.

For example, `100::0xabcdef0123456789` or `1.23::POL`";

/// Returns a tracked Account ID matching a hex string or the default one defined in the Client
/// config.
pub(crate) async fn get_input_acc_id_by_prefix_or_default(
    client: &Client,
    account_id: Option<String>,
) -> Result<AccountId, CliError> {
    let account_id_str = if let Some(account_id_prefix) = account_id {
        account_id_prefix
    } else {
        let (cli_config, _) = load_config_file()?;

        cli_config
            .default_account_id
            .ok_or(CliError::Input("No input account ID nor default account defined".to_string()))?
    };

    parse_account_id(client, &account_id_str).await
}

/// Parses a user provided account ID string and returns the corresponding `AccountId`.
///
/// `account_id` can fall into three categories:
///
/// - It's a hex prefix of an account ID of an account tracked by the client.
/// - It's a full hex account ID.
/// - It's a full bech32 account ID.
///
/// # Errors
///
/// - Will return a `IdPrefixFetchError` if the provided account ID string can't be parsed as an
///   `AccountId` and doesn't correspond to an account tracked by the client either.
pub(crate) async fn parse_account_id(
    client: &Client,
    account_id: &str,
) -> Result<AccountId, CliError> {
    if account_id.starts_with("0x") {
        if let Ok(account_id) = AccountId::from_hex(account_id) {
            return Ok(account_id);
        }

        Ok(get_account_with_id_prefix(client, account_id)
        .await
        .map_err(|_| CliError::Input(format!("Input account ID {account_id} is neither a valid Account ID nor a hex prefix of a known Account ID")))?
        .id())
    } else {
        Ok(AccountId::from_bech32(account_id)
            .map_err(|_| {
                CliError::Input(format!(
                    "Input account ID {account_id} is not a valid bech32 encoded Account ID"
                ))
            })?
            .1)
    }
}

pub(crate) fn update_config(config_path: &Path, client_config: &CliConfig) -> Result<(), CliError> {
    let config_as_toml_string = toml::to_string_pretty(&client_config).map_err(|err| {
        CliError::Config("Failed to parse config file as TOML".to_string().into(), err.to_string())
    })?;

    info!("Writing config file at: {:?}", config_path);
    let mut file_handle = File::options().write(true).truncate(true).open(config_path)?;

    file_handle.write_all(config_as_toml_string.as_bytes())?;

    println!("Config updated successfully");
    Ok(())
}

/// Loads config file from current directory and default filename and returns it alongside its path.
///
/// This function will look for the configuration file at the provided path. If the path is
/// relative, searches in parent directories all the way to the root as well.
pub(super) fn load_config_file() -> Result<(CliConfig, PathBuf), CliError> {
    let mut current_dir = std::env::current_dir()?;
    current_dir.push(CLIENT_CONFIG_FILE_NAME);
    let config_path = current_dir.as_path();

    let cli_config = load_config(config_path)?;

    Ok((cli_config, config_path.into()))
}

/// Loads the client configuration.
fn load_config(config_file: &Path) -> Result<CliConfig, CliError> {
    Figment::from(Toml::file(config_file)).extract().map_err(|err| {
        CliError::Config("Failed to load config file".to_string().into(), err.to_string())
    })
}

/// Returns the faucet details map using the config file.
pub fn load_faucet_details_map() -> Result<FaucetDetailsMap, CliError> {
    let (config, _) = load_config_file()?;
    FaucetDetailsMap::new(config.token_symbol_map_filepath)
}
