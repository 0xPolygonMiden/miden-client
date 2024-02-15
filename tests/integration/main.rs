use miden_client::client::accounts::{AccountStorageMode, AccountTemplate};
use std::env::temp_dir;
use std::fs;
use uuid::Uuid;

use objects::accounts::AccountData;
use objects::assets::{Asset, FungibleAsset};
use objects::utils::serde::Deserializable;

use miden_client::client::transactions::{PaymentTransactionData, TransactionTemplate};
use miden_client::client::Client;
use miden_client::config::{ClientConfig, RpcConfig};
use miden_client::store::{notes::InputNoteFilter, transactions::TransactionFilter};

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
const TRANSFER_AMOUNT: u64 = 50;

#[tokio::main]
async fn main() {
    let mut client = create_test_client();

    // Enusre clean state
    assert!(client.get_accounts().unwrap().is_empty());
    assert!(client
        .get_transactions(TransactionFilter::All)
        .unwrap()
        .is_empty());
    assert!(client
        .get_input_notes(InputNoteFilter::All)
        .unwrap()
        .is_empty());

    // Import accounts
    println!("Importing Accounts...");
    for account_idx in 0..2 {
        let account_data_file_contents =
            fs::read(format!("./miden-node/accounts/account{}.mac", account_idx)).unwrap();
        let account_data = AccountData::read_from_bytes(&account_data_file_contents).unwrap();
        client.import_account(account_data).unwrap();
    }

    // Create new regular account
    client
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    println!("Syncing State...");
    client.sync_state().await.unwrap();

    // Get Faucet and regular accounts
    println!("Fetching Accounts...");
    let accounts = client.get_accounts().unwrap();
    let regular_account_stubs = accounts
        .iter()
        .filter(|(account, _seed)| account.id().is_regular_account())
        .map(|(account, _seed)| account)
        .collect::<Vec<_>>();
    let (faucet_account_stub, _seed) = accounts
        .iter()
        .find(|(account, _seed)| !account.id().is_regular_account())
        .unwrap();
    assert_eq!(accounts.len(), 3);

    let first_regular_account_id = regular_account_stubs[0].id();
    let second_regular_account_id = regular_account_stubs[1].id();
    let faucet_account_id = faucet_account_stub.id();

    let (regular_account, _seed) = client.get_account_by_id(first_regular_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 0);

    // Create a Mint Tx for 1000 units of our fungible asset
    let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::MintFungibleAsset {
        asset: fungible_asset,
        target_account_id: first_regular_account_id,
    };
    println!("Minting Asset");
    execute_tx_and_sync(&mut client, tx_template).await;

    // Check that note is committed
    println!("Fetching Pending Notes...");
    let notes = client.get_input_notes(InputNoteFilter::Pending).unwrap();
    assert!(notes.is_empty());

    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(InputNoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    let tx_template =
        TransactionTemplate::ConsumeNotes(first_regular_account_id, vec![notes[0].note_id()]);
    println!("Consuming Note...");
    execute_tx_and_sync(&mut client, tx_template).await;

    let (regular_account, _seed) = client.get_account_by_id(first_regular_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), MINT_AMOUNT);
    } else {
        panic!("ACCOUNT SHOULD HAVE A FUNGIBLE ASSET");
    }

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(PaymentTransactionData::new(
        Asset::Fungible(asset),
        first_regular_account_id,
        second_regular_account_id,
    ));
    println!("Running P2ID tx...");
    execute_tx_and_sync(&mut client, tx_template).await;

    // Check that note is committed for the second account to consume
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(InputNoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    // Consume P2ID note
    let tx_template =
        TransactionTemplate::ConsumeNotes(second_regular_account_id, vec![notes[0].note_id()]);
    println!("Consuming Note...");
    execute_tx_and_sync(&mut client, tx_template).await;

    let (regular_account, _seed) = client.get_account_by_id(first_regular_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    // Validate the transfered amounts
    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), MINT_AMOUNT - TRANSFER_AMOUNT);
    } else {
        panic!("ACCOUNT SHOULD HAVE A FUNGIBLE ASSET");
    }

    let (regular_account, _seed) = client.get_account_by_id(second_regular_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), TRANSFER_AMOUNT);
    } else {
        panic!("ACCOUNT SHOULD HAVE A FUNGIBLE ASSET");
    }

    println!("Test ran successfully!");
}
