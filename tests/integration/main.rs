use miden_client::client::Client;
use miden_client::client::{
    accounts::{AccountStorageMode, AccountTemplate},
    rpc::TonicRpcClient,
    transactions::PaymentTransactionData,
    transactions::TransactionTemplate,
};
use miden_client::config::{ClientConfig, RpcConfig};
use miden_client::errors::{ClientError, NodeRpcClientError};
use miden_client::store::{sqlite_store::SqliteStore, NoteFilter, TransactionFilter};

use miden_tx::{DataStoreError, TransactionExecutorError};
use objects::{
    accounts::AccountData,
    assets::{Asset, FungibleAsset},
    utils::serde::Deserializable,
};

use std::env::temp_dir;
use std::fs;
use std::time::Duration;

use uuid::Uuid;

type TestClient = Client<TonicRpcClient, SqliteStore>;

fn create_test_client() -> TestClient {
    let client_config = ClientConfig {
        store: create_test_store_path()
            .into_os_string()
            .into_string()
            .unwrap()
            .try_into()
            .unwrap(),
        rpc: RpcConfig::default(),
    };

    let rpc_endpoint = client_config.rpc.endpoint.to_string();
    let store = SqliteStore::new((&client_config).into()).unwrap();
    // TODO: See if we can solve this by wrapping store with a `Rc<Cell<..>>` or a `Rc<RefCell<..>>`
    let data_store_store = SqliteStore::new((&client_config).into()).unwrap();
    TestClient::new(TonicRpcClient::new(&rpc_endpoint), store, data_store_store).unwrap()
}

fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}

async fn execute_tx_and_sync(client: &mut TestClient, tx_template: TransactionTemplate) {
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
    while client.sync_state().await.unwrap() <= current_block_num + 1 {
        std::thread::sleep(std::time::Duration::new(3, 0));
    }
}

/// Waits for node to be running.
///
/// # Panics
///
/// This function will panic if it does `NUMBER_OF_NODE_ATTEMPTS` unsuccessful checks or if we
/// receive an error other than a connection related error
async fn wait_for_node(client: &mut TestClient) {
    const NODE_TIME_BETWEEN_ATTEMPTS: u64 = 5;
    const NUMBER_OF_NODE_ATTEMPTS: u64 = 60;

    println!("Waiting for Node to be up. Checking every {NODE_TIME_BETWEEN_ATTEMPTS}s for {NUMBER_OF_NODE_ATTEMPTS} tries...");

    for _try_number in 0..NUMBER_OF_NODE_ATTEMPTS {
        match client.sync_state().await {
            Err(ClientError::NodeRpcClientError(NodeRpcClientError::ConnectionError(_))) => {
                std::thread::sleep(Duration::from_secs(NODE_TIME_BETWEEN_ATTEMPTS));
            }
            Err(other_error) => {
                panic!("Unexpected error: {other_error}");
            }
            _ => return,
        }
    }

    panic!("Unable to connect to node");
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
    assert!(client.get_input_notes(NoteFilter::All).unwrap().is_empty());

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

    wait_for_node(&mut client).await;

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

    let (regular_account, _seed) = client.get_account(first_regular_account_id).unwrap();
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
    let notes = client.get_input_notes(NoteFilter::Pending).unwrap();
    assert!(notes.is_empty());

    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    let tx_template =
        TransactionTemplate::ConsumeNotes(first_regular_account_id, vec![notes[0].note_id()]);
    println!("Consuming Note...");
    execute_tx_and_sync(&mut client, tx_template).await;

    let (regular_account, _seed) = client.get_account(first_regular_account_id).unwrap();

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
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    // Consume P2ID note
    let tx_template =
        TransactionTemplate::ConsumeNotes(second_regular_account_id, vec![notes[0].note_id()]);
    println!("Consuming Note...");
    execute_tx_and_sync(&mut client, tx_template).await;

    let (regular_account, seed) = client.get_account(first_regular_account_id).unwrap();
    // The seed should not be retrieved due to the account not being new
    assert!(!regular_account.is_new() && seed.is_none());
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    // Validate the transfered amounts
    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), MINT_AMOUNT - TRANSFER_AMOUNT);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    let (regular_account, _seed) = client.get_account(second_regular_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), TRANSFER_AMOUNT);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    // Check that we can't consume the P2ID note again
    let tx_template =
        TransactionTemplate::ConsumeNotes(second_regular_account_id, vec![notes[0].note_id()]);
    println!("Consuming Note...");

    // Double-spend error expected to be received since we are consuming the same note
    match client.new_transaction(tx_template) {
        Err(ClientError::TransactionExecutionError(
            TransactionExecutorError::FetchTransactionInputsFailed(
                DataStoreError::NoteAlreadyConsumed(_),
            ),
        )) => {}
        Ok(_) => panic!("Double-spend error: Note should not be consumable!"),
        _ => panic!("Unexpected error: {}", notes[0].note_id().to_hex()),
    }

    println!("Test ran successfully!");
}
