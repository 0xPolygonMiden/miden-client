use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use crypto::{dsa::rpo_falcon512::KeyPair, utils::Deserializable};
use miden_client::{client::Client, config::ClientConfig, store::accounts::AuthInfo};
use miden_node_store::genesis::GenesisState;
use objects::accounts::{AccountData, AuthData};

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
    /// Insert data from node's genesis file
    LoadGenesis {
        /// The directory that contains the files generated from the node
        #[clap(short, long)]
        genesis_path: PathBuf,
        /// Optionally decide which indices are imported (indices are zero-based)
        #[clap()]
        account_indices: Option<Vec<usize>>,
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
            Command::MockData { transaction: _ } => {
                let _client = client;
                // miden_client::mock::insert_mock_data(&mut client);
                // if *transaction {
                //     miden_client::mock::create_mock_transaction(&mut client).await;
                // }
                Ok(())
            }
            Command::LoadGenesis {
                genesis_path,
                account_indices,
            } => {
                let mut client = client;
                load_genesis_data(&mut client, genesis_path, account_indices.clone())
            }
        }
    }
}

pub fn load_genesis_data(
    client: &mut Client,
    path: &Path,
    account_indices: Option<Vec<usize>>,
) -> Result<(), String> {
    let file_contents = fs::read(path.join("genesis.dat")).map_err(|err| err.to_string())?;

    let genesis_state =
        GenesisState::read_from_bytes(&file_contents).map_err(|err| err.to_string())?;

    let range = if let Some(indices) = account_indices {
        indices
    } else {
        (0..genesis_state.accounts.len()).collect()
    };

    for account_index in range {
        let account_data_filepath = format!("accounts/account{}.mac", account_index);
        let account_data_file_contents =
            fs::read(path.join(account_data_filepath)).map_err(|err| err.to_string())?;
        let account_data = AccountData::read_from_bytes(&account_data_file_contents)
            .map_err(|err| err.to_string())?;

        match account_data.auth {
            AuthData::RpoFalcon512Seed(key_pair) => {
                let keypair = KeyPair::from_seed(&key_pair).map_err(|err| err.to_string())?;
                let seed = account_data
                    .account_seed
                    .ok_or("Account seed was expected")?;

                client
                    .insert_account(
                        &account_data.account,
                        seed,
                        &AuthInfo::RpoFalcon512(keypair),
                    )
                    .map_err(|err| err.to_string())?;
            }
        }
    }
    Ok(())
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use super::{Cli, Command};
    use std::{env::temp_dir, fs, path::PathBuf, thread, time::Duration};

    use crypto::{utils::Serializable, Felt, FieldElement};
    use miden_client::{
        client::Client,
        config::{ClientConfig, Endpoint},
    };
    use miden_lib::transaction::TransactionKernel;
    use miden_node_store::genesis::GenesisState;
    use mock::{
        constants::{generate_account_seed, AccountSeedType},
        mock::account,
    };
    use objects::accounts::{Account, AccountData, AuthData};
    use rand::{rngs::ThreadRng, thread_rng, Rng};

    fn create_account_data(
        rng: &mut ThreadRng,
        seed_type: AccountSeedType,
        account_file_path: PathBuf,
    ) -> Account {
        // Create an account and save it to a file
        let (account_id, account_seed) = generate_account_seed(seed_type);
        let assembler = TransactionKernel::assembler();
        let account = account::mock_account(Some(account_id.into()), Felt::ZERO, None, &assembler);

        let key_pair_seed: [u32; 10] = rng.gen();
        let mut key_pair_seed_u8: [u8; 40] = [0; 40];
        for (dest_c, source_e) in key_pair_seed_u8
            .chunks_exact_mut(4)
            .zip(key_pair_seed.iter())
        {
            dest_c.copy_from_slice(&source_e.to_le_bytes())
        }
        let auth_data = AuthData::RpoFalcon512Seed(key_pair_seed_u8);

        let account_data = AccountData::new(account.clone(), Some(account_seed), auth_data);
        fs::write(account_file_path, account_data.to_bytes()).unwrap();

        account
    }

    fn reset_db() -> PathBuf {
        const STORE_FILENAME: &str = "test.store.sqlite3";

        // get directory of the currently executing binary, or fallback to the current directory
        let exec_dir = match std::env::current_exe() {
            Ok(mut path) => {
                path.pop();
                path
            }
            Err(_) => PathBuf::new(),
        };

        let store_path = exec_dir.join(STORE_FILENAME);
        if store_path.exists() {
            fs::remove_file(&store_path).unwrap();
            thread::sleep(Duration::from_secs(1));
        }

        store_path
    }

    pub fn create_genesis_data() -> (PathBuf, Vec<Account>) {
        let temp_dir = temp_dir();
        let mut rng = thread_rng();

        let account_dir = temp_dir.join("accounts");
        fs::create_dir_all(account_dir.clone()).unwrap();

        let account = create_account_data(
            &mut rng,
            AccountSeedType::RegularAccountUpdatableCodeOnChain,
            account_dir.join("account0.mac"),
        );

        // Create a Faucet and save it to a file
        let faucet_account = create_account_data(
            &mut rng,
            AccountSeedType::FungibleFaucetValidInitialBalance,
            account_dir.join("account1.mac"),
        );

        // Create Genesis state and save it to a file
        let accounts = vec![account, faucet_account];
        let genesis_state = GenesisState::new(accounts.clone(), 1, 1);
        fs::write(temp_dir.join("genesis.dat"), genesis_state.to_bytes()).unwrap();

        (temp_dir, accounts)
    }

    #[tokio::test]
    async fn load_genesis_test() {
        let store_path = reset_db();
        let (genesis_data_path, created_accounts) = create_genesis_data();
        let load_genesis_command = Command::LoadGenesis {
            genesis_path: genesis_data_path,
            account_indices: None,
        };
        let cli = Cli {
            action: load_genesis_command,
        };
        cli.execute().await.unwrap();

        let client = Client::new(ClientConfig::new(
            store_path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
        ))
        .await
        .unwrap();

        // TODO: make create_genesis_data at least return the ids of the accounts to make a better
        // check
        let accounts = client.get_accounts().unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].0.id(), created_accounts[0].id());
        assert_eq!(accounts[1].0.id(), created_accounts[1].id());
    }
}
