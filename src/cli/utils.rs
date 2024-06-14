use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use figment::{
    providers::{Format, Toml},
    Figment,
};
use miden_client::{
    client::{rpc::NodeRpcClient, Client},
    config::ClientConfig,
    store::Store,
};
use miden_objects::{
    accounts::AccountId,
    crypto::rand::FeltRng,
    notes::{NoteExecutionHint, NoteTag, NoteType},
    NoteError,
};
use miden_tx::auth::TransactionAuthenticator;

use super::{get_account_with_id_prefix, CLIENT_CONFIG_FILE_NAME};
use crate::cli::info;

/// Returns a tracked Account ID matching a hex string or the default one defined in the Client config
pub(crate) fn get_input_acc_id_by_prefix_or_default<
    N: NodeRpcClient,
    R: FeltRng,
    S: Store,
    A: TransactionAuthenticator,
>(
    client: &Client<N, R, S, A>,
    account_id: Option<String>,
) -> Result<AccountId, String> {
    let account_id_str = if let Some(account_id_prefix) = account_id {
        account_id_prefix
    } else {
        let (miden_client_config, _) = load_config_file()?;

        let default_account_id = miden_client_config
            .cli
            .ok_or("No CLI config found in the configuration file")?
            .default_account_id;

        default_account_id.ok_or("No input account ID nor default account defined")?
    };

    parse_account_id(client, &account_id_str)
}

/// Parses a user provided account id string and returns the corresponding `AccountId`
///
/// `account_id` can fall into two categories:
///
/// - it's a prefix of an account id of an account tracked by the client
/// - it's a full account id
///
/// # Errors
///
/// - Will return a `IdPrefixFetchError` if the provided account id string can't be parsed as an
///   `AccountId` and does not correspond to an account tracked by the client either.
pub(crate) fn parse_account_id<
    N: NodeRpcClient,
    R: FeltRng,
    S: Store,
    A: TransactionAuthenticator,
>(
    client: &Client<N, R, S, A>,
    account_id: &str,
) -> Result<AccountId, String> {
    if let Ok(account_id) = AccountId::from_hex(account_id) {
        return Ok(account_id);
    }

    let account_id = get_account_with_id_prefix(client, account_id)
    .map_err(|_err| "Input account ID {account_id} is neither a valid Account ID nor a prefix of a known Account ID")?
    .id();
    Ok(account_id)
}

pub(crate) fn update_config(config_path: &Path, client_config: ClientConfig) -> Result<(), String> {
    let config_as_toml_string = toml::to_string_pretty(&client_config)
        .map_err(|err| format!("error formatting config: {err}"))?;

    info!("Writing config file at: {:?}", config_path);
    let mut file_handle = File::options()
        .write(true)
        .truncate(true)
        .open(config_path)
        .map_err(|err| format!("error opening the file: {err}"))?;

    file_handle
        .write(config_as_toml_string.as_bytes())
        .map_err(|err| format!("error writing to file: {err}"))?;

    println!("Config updated successfully");
    Ok(())
}

/// Returns a note tag for a swap note with the specified parameters.
///
/// Use case ID for the returned tag is set to 0.
///
/// Tag payload is constructed by taking asset tags (8 bits of faucet ID) and concatenating them
/// together as offered_asset_tag + requested_asset tag.
///
/// Network execution hint for the returned tag is set to `Local`.
///
/// Based on miden-base's implementation (<https://github.com/0xPolygonMiden/miden-base/blob/9e4de88031b55bcc3524cb0ccfb269821d97fb29/miden-lib/src/notes/mod.rs#L153>)
///
/// TODO: we should make the function in base public and once that gets released use that one and
/// delete this implementation.
pub fn build_swap_tag(
    note_type: NoteType,
    offered_asset_faucet_id: AccountId,
    requested_asset_faucet_id: AccountId,
) -> Result<NoteTag, NoteError> {
    const SWAP_USE_CASE_ID: u16 = 0;

    // get bits 4..12 from faucet IDs of both assets, these bits will form the tag payload; the
    // reason we skip the 4 most significant bits is that these encode metadata of underlying
    // faucets and are likely to be the same for many different faucets.

    let offered_asset_id: u64 = offered_asset_faucet_id.into();
    let offered_asset_tag = (offered_asset_id >> 52) as u8;

    let requested_asset_id: u64 = requested_asset_faucet_id.into();
    let requested_asset_tag = (requested_asset_id >> 52) as u8;

    let payload = ((offered_asset_tag as u16) << 8) | (requested_asset_tag as u16);

    let execution = NoteExecutionHint::Local;
    match note_type {
        NoteType::Public => NoteTag::for_public_use_case(SWAP_USE_CASE_ID, payload, execution),
        _ => NoteTag::for_local_use_case(SWAP_USE_CASE_ID, payload),
    }
}

/// Loads config file from current directory and default filename and returns it alongside its path
///
/// This function will look for the configuration file at the provided path. If the path is
/// relative, searches in parent directories all the way to the root as well.
pub(super) fn load_config_file() -> Result<(ClientConfig, PathBuf), String> {
    let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
    current_dir.push(CLIENT_CONFIG_FILE_NAME);
    let config_path = current_dir.as_path();

    let client_config = load_config(config_path)?;

    Ok((client_config, config_path.into()))
}

/// Loads the client configuration.
fn load_config(config_file: &Path) -> Result<ClientConfig, String> {
    Figment::from(Toml::file(config_file))
        .extract()
        .map_err(|err| format!("Failed to load {} config file: {err}", config_file.display()))
}

/// Parses a fungible Asset and returns it as a tuple of the amount and the faucet account ID hex.
///
/// TODO: currently we'll only parse AccountId, however once we tackle
/// [#258](https://github.com/0xPolygonMiden/miden-client/issues/258) we should also add the
/// possibility to parse account aliases / token symbols dependeing on the path we choose.
///
/// # Errors
///
/// Will return an error if the provided `arg` doesn't match one of the expected format:
///
/// - `<AMOUNT>::<FAUCET_ID>`, such as `100::0x123456789`
pub fn parse_fungible_asset(arg: &str) -> Result<(u64, AccountId), String> {
    let (amount, faucet) = arg.split_once("::").ok_or("Separator `::` not found!")?;
    let amount = amount.parse::<u64>().map_err(|err| err.to_string())?;
    let faucet_id = AccountId::from_hex(faucet).map_err(|err| err.to_string())?;

    Ok((amount, faucet_id))
}
