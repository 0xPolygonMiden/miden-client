use std::env::temp_dir;
use std::fs;
use uuid::Uuid;

use objects::accounts::AccountData;
use objects::assets::{Asset, FungibleAsset};
use objects::utils::serde::Deserializable;

use miden_client::client::transactions::TransactionTemplate;
use miden_client::client::Client;
use miden_client::config::{ClientConfig, RpcConfig};
use miden_client::store::{notes::NoteFilter, transactions::TransactionFilter};

fn create_test_client() -> Client {
    let client_config = ClientConfig {
        store: create_test_store_path()
            .into_os_string()
            .into_string()
            .unwrap()
            .try_into()
            .unwrap(),
        rpc: RpcConfig::default(),
    };

    Client::new(client_config).unwrap()
}

fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}

async fn execute_tx_and_sync(client: &mut Client, tx_template: TransactionTemplate) {
    println!("Executing Transaction");
    let transaction_execution_result = client.new_transaction(tx_template).unwrap();

    println!("Sending Transaction to node");
    client
        .send_transaction(transaction_execution_result)
        .await
        .unwrap();

    let current_block_num = client.sync_state().await.unwrap();

    // Wait until we've actually gotten a new block
    println!("Syncing State...");
    while client.sync_state().await.unwrap() == current_block_num {
        std::thread::sleep(std::time::Duration::new(5, 0));
    }
}

const MINT_AMOUNT: u64 = 1000;

#[tokio::main]
async fn main() {
    let mut client = create_test_client();

    // Enusre clean state
    assert!(client.get_accounts().unwrap().is_empty());
    assert!(client
        .get_transactions(TransactionFilter::All)
        .unwrap()
        .is_empty());
    assert!(client.get_input_notes(NoteFilter::All).unwrap().is_empty());

    // Import accounts
    println!("Importing Accounts...");
    {
        let account_data_file_contents = fs::read("./miden-node/accounts/account0.mac").unwrap();
        let account_data = AccountData::read_from_bytes(&account_data_file_contents).unwrap();
        client.import_account(account_data).unwrap();
    }

    {
        let account_data_file_contents = fs::read("./miden-node/accounts/account1.mac").unwrap();
        let account_data = AccountData::read_from_bytes(&account_data_file_contents).unwrap();
        client.import_account(account_data).unwrap();
    }

    println!("Syncing State...");
    client.sync_state().await.unwrap();

    // Get Faucet and regular accounts
    println!("Fetching Accounts...");
    let accounts = client.get_accounts().unwrap();
    let (regular_account_stub, _seed) = accounts
        .iter()
        .find(|(account, _seed)| account.id().is_regular_account())
        .unwrap();
    let (faucet_account_stub, _seed) = accounts
        .iter()
        .find(|(account, _seed)| !account.id().is_regular_account())
        .unwrap();
    assert_eq!(accounts.len(), 2);

    let regular_account_id = regular_account_stub.id();
    let faucet_account_id = faucet_account_stub.id();

    let (regular_account, _seed) = client.get_account_by_id(regular_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 0);

    // Create a Mint Tx for 1000 units of our fungible asset
    let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::MintFungibleAsset {
        asset: fungible_asset,
        target_account_id: regular_account_id,
    };
    println!("Minting Asset");
    execute_tx_and_sync(&mut client, tx_template).await;

    // Check that note is committed
    println!("Fetching Pending Notes...");
    let notes = client.get_input_notes(NoteFilter::Pending).unwrap();
    assert!(notes.is_empty());

    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    let tx_template =
        TransactionTemplate::ConsumeNotes(regular_account_id, vec![notes[0].note_id()]);
    println!("Consuming Note...");
    execute_tx_and_sync(&mut client, tx_template).await;

    let (regular_account, _seed) = client.get_account_by_id(regular_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), MINT_AMOUNT);
    } else {
        panic!("ACCOUNT SHOULD HAVE A FUNGIBLE ASSET");
    }

    println!("Test ran successfully!");
}
