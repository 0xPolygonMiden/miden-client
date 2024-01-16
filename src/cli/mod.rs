use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use crypto::{dsa::rpo_falcon512::KeyPair, utils::Deserializable};
use miden_client::{client::Client, config::ClientConfig, store::accounts::AuthInfo};
use miden_node_store::genesis::GenesisState;

mod account;
mod input_notes;
mod sync_state;
mod transactions;

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(
    name = "Miden",
    about = "Miden Client",
    version,
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[clap(subcommand)]
    action: Command,
}

/// CLI actions
#[derive(Debug, Parser)]
pub enum Command {
    #[clap(subcommand)]
    Account(account::AccountCmd),
    #[clap(subcommand)]
    InputNotes(input_notes::InputNotes),
    #[clap(subcommand)]
    SyncState(sync_state::SyncStateCmd),
    #[clap(subcommand)]
    Transaction(transactions::Transaction),
    #[cfg(feature = "mock")]
    /// Insert mock data into the client. This is optional because it takes a few seconds
    MockData {
        #[clap(short, long)]
        transaction: bool,
    },
    //#[cfg(feature = "testing")]
    /// Insert data from node's genesis file
    LoadGenesis {
        /// The directory that contains the three files generated from the node: genesis.dat, faucet.fsk and wallet.fs
        #[clap(short, long)]
        genesis_path: PathBuf,
    },
}

/// CLI entry point
impl Cli {
    pub async fn execute(&self) -> Result<(), String> {
        // create a client
        let client = Client::new(ClientConfig::default())
            .await
            .map_err(|err| err.to_string())?;

        // execute cli command
        match &self.action {
            Command::Account(account) => account.execute(client),
            Command::InputNotes(notes) => notes.execute(client),
            Command::SyncState(tags) => tags.execute(client).await,
            Command::Transaction(transaction) => transaction.execute(client).await,
            #[cfg(feature = "mock")]
            Command::MockData { transaction } => {
                let mut client = client;
                miden_client::mock::insert_mock_data(&mut client).await;
                if *transaction {
                    miden_client::mock::create_mock_transaction(&mut client).await;
                }
                Ok(())
            }
            Command::LoadGenesis { genesis_path } => {
                let mut client = client;
                load_genesis_data(&mut client, genesis_path)
            }
        }
    }
}

pub fn load_genesis_data(client: &mut Client, path: &Path) -> Result<(), String> {
    let file_contents = fs::read(path.join("genesis.dat")).map_err(|err| err.to_string())?;

    let genesis_state =
        GenesisState::read_from_bytes(&file_contents).map_err(|err| err.to_string())?;

    if genesis_state.accounts.len() != 2 {
        return Err(format!(
            "error: genesis state file should have 2 accounts, has {}",
            genesis_state.accounts.len()
        ));
    }

    for acc_and_seed in genesis_state.accounts {
        let account = acc_and_seed.account;
        let seed = acc_and_seed.seed;

        let key_pair = if account.is_faucet() {
            let file_contents = fs::read(path.join("faucet.fsk")).unwrap();
            let _ = account.code().procedure_tree();

            KeyPair::read_from_bytes(&file_contents).unwrap()
        } else {
            let file_contents = fs::read(path.join("wallet.fsk")).unwrap();
            KeyPair::read_from_bytes(&file_contents).unwrap()
        };
        client
            .insert_account(&account, seed, &AuthInfo::RpoFalcon512(key_pair))
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}
