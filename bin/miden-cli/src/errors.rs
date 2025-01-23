use std::error::Error as StdError;

use miden_client::ClientError;
use miden_objects::{AccountError, AccountIdError, AssetError};
use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum CliError {
    #[error("account error: {1} with error {0}")]
    #[diagnostic(code(cli::account_error))]
    Account(#[source] AccountError, String),
    #[error("account id error: {0} with error {1}")]
    #[diagnostic(code(cli::accountid_error), help("Check the account ID format."))]
    AccountId(#[source] AccountIdError, String),
    #[error("asset error: {0}")]
    #[diagnostic(code(cli::asset_error))]
    Asset(#[source] AssetError),
    #[error("client error")]
    #[diagnostic(code(cli::client_error))]
    Client(#[from] ClientError),
    #[error("config error: {1} with error {0}")]
    #[diagnostic(
        code(cli::config_error),
        help("Check if the configuration file exists and is well-formed.")
    )]
    Config(Box<dyn StdError + Send + Sync>, String),
    #[error("export error: {0}")]
    #[diagnostic(code(cli::export_error), help("Check the ID."))]
    Export(String),
    #[error("faucet error: {0}")]
    #[diagnostic(code(cli::faucet_error))]
    Faucet(String),
    #[error("import error: {0}")]
    #[diagnostic(code(cli::import_error), help("Check the file name."))]
    Import(String),
    #[error("input error: {0}")]
    #[diagnostic(code(cli::input_error))]
    Input(String),
    #[error("io error")]
    #[diagnostic(code(cli::io_error))]
    IO(#[from] std::io::Error),
    #[error("missing flag: {0}")]
    #[diagnostic(code(cli::config_error), help("Check the configuration file format."))]
    MissingFlag(String),
    #[error("parse error: {0} with error {1}")]
    #[diagnostic(code(cli::parse_error), help("Check the inputs."))]
    Parse(String, String),
    #[error("transaction error: {0} with error {1}")]
    #[diagnostic(code(cli::transaction_error))]
    Transaction(String, String),
}

impl From<CliError> for String {
    fn from(err: CliError) -> String {
        err.to_string()
    }
}
