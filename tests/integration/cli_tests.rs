use std::{
    env::{set_current_dir, temp_dir},
    path::Path,
    rc::Rc,
};

use assert_cmd::Command;
use miden_client::{
    client::{get_random_coin, rpc::TonicRpcClient, store_authenticator::StoreAuthenticator},
    config::ClientConfig,
    store::{sqlite_store::SqliteStore, NoteFilter},
};

use crate::{create_test_store_path, TestClient};

// INIT TESTS
// ================================================================================================

#[test]
fn test_init_without_params() {
    // For now sleep to ensure node's up
    std::thread::sleep(std::time::Duration::new(30, 0));

    let mut temp_dir = temp_dir();
    temp_dir.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir.clone()).unwrap();
    set_current_dir(temp_dir).unwrap();

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init"]);
    init_cmd.assert().success();

    sync_cli()
}

#[test]
fn test_init_with_params() {
    // For now sleep to ensure node's up
    std::thread::sleep(std::time::Duration::new(30, 0));

    let store_path = create_test_store_path();
    let mut temp_dir = temp_dir();
    temp_dir.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir.clone()).unwrap();
    set_current_dir(temp_dir).unwrap();

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--rpc", "localhost", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.assert().success();

    sync_cli()
}

// IMPORT TESTS
// ================================================================================================

// Accounts 0 and 1 should be basic wallets and account2 should be a fungible faucet
const GENESIS_ACCOUNTS_FILENAMES: [&str; 3] = ["account0.mac", "account1.mac", "account2.mac"];
const GENESIS_ACCOUNTS_IDS: [&str; 3] =
    ["0x8add712899d6ab76", "0x86bac4a17250e9f6", "0xa1834e02152a0f08"];

// This tests that it's possible to import the genesis accounts and interact with them. To do so it:
//
// 1. Creates a new client
// 2. Imports all 3 genesis accounts
// 3. Runs a mint tx, syncs and consumes the created note with none of the regular accounts
// 4. Runs a P2ID tx from the account that just received the asset to the remaining basic account
// 5. Syncs and consumes the P2ID note with the other account
#[test]
fn test_import_genesis_accounts_can_be_used_for_transactions() {
    let first_basic_account_id = GENESIS_ACCOUNTS_IDS[0];
    let second_basic_account_id = GENESIS_ACCOUNTS_IDS[1];
    let fungible_faucet_account_id = GENESIS_ACCOUNTS_IDS[2];
    // For now sleep to ensure node's up
    std::thread::sleep(std::time::Duration::new(30, 0));

    let store_path = create_test_store_path();
    let mut temp_dir = temp_dir();
    temp_dir.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir.clone()).unwrap();

    for genesis_account_filename in GENESIS_ACCOUNTS_FILENAMES {
        let mut new_file_path = temp_dir.clone();
        new_file_path.push(genesis_account_filename);
        std::fs::copy(format!("./miden-node/accounts/{}", genesis_account_filename), new_file_path)
            .unwrap();
    }

    set_current_dir(temp_dir).unwrap();

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.assert().success();

    // Import genesis accounts
    let mut args = vec!["import"];
    for filename in GENESIS_ACCOUNTS_FILENAMES {
        args.push(filename);
    }
    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    import_cmd.args(&args);
    import_cmd.assert().success();

    sync_cli();

    // Ensure they've been importing by showing them
    // TODO: Once show is fixed for faucet account do the full iteration without skipping the
    // faucet
    for account_id in &GENESIS_ACCOUNTS_IDS[..=1] {
        let args = vec!["account", "--show", account_id];
        let mut show_cmd = Command::cargo_bin("miden").unwrap();
        show_cmd.args(&args);
        show_cmd.assert().success();
    }

    // Let's try and mint
    let mut mint_cmd = Command::cargo_bin("miden").unwrap();
    mint_cmd.args([
        "mint",
        "--target",
        first_basic_account_id,
        "--faucet",
        fungible_faucet_account_id,
        "--amount",
        "100",
        "-n",
        "private",
        "--force",
    ]);
    mint_cmd.assert().success();

    // Sleep for a while to ensure the note is committed on the node
    std::thread::sleep(std::time::Duration::new(15, 0));
    sync_cli();

    // Consume the note
    let note_to_consume_id = {
        let client = create_test_client_with_store_path(&store_path);
        let notes = client.get_input_notes(NoteFilter::Committed).unwrap();

        notes.first().unwrap().id().to_hex()
    };

    let mut consume_note_cmd = Command::cargo_bin("miden").unwrap();
    consume_note_cmd.args([
        "consume-notes",
        "--account",
        first_basic_account_id,
        "--force",
        &note_to_consume_id,
    ]);
    consume_note_cmd.assert().success();

    // Sleep for a while to ensure the consumption is done on the node
    std::thread::sleep(std::time::Duration::new(15, 0));
    sync_cli();

    // Send assets to second account
    let mut p2id_cmd = Command::cargo_bin("miden").unwrap();
    p2id_cmd.args([
        "send",
        "--sender",
        first_basic_account_id,
        "--target",
        second_basic_account_id,
        "--faucet",
        fungible_faucet_account_id,
        "-n",
        "private",
        "--force",
        "25",
    ]);
    p2id_cmd.assert().success();

    // Sleep for a while to ensure the consumption is done on the node
    std::thread::sleep(std::time::Duration::new(15, 0));
    sync_cli();

    // Consume note for second account
    let note_to_consume_id = {
        let client = create_test_client_with_store_path(&store_path);
        let notes = client.get_input_notes(NoteFilter::Committed).unwrap();

        notes.first().unwrap().id().to_hex()
    };

    let mut consume_note_cmd = Command::cargo_bin("miden").unwrap();
    consume_note_cmd.args([
        "consume-notes",
        "--account",
        second_basic_account_id,
        "--force",
        &note_to_consume_id,
    ]);
    consume_note_cmd.assert().success();

    // Sleep for a while to ensure the consumption is done on the node
    std::thread::sleep(std::time::Duration::new(15, 0));
    sync_cli();
}

// This tests that it's possible to export and import accounts into other CLIs. To do so it:
//
// 1. Creates a client A with a faucet
// 2. Creates a client B with a regular account
// 3. On client A runs a mint transaction, and exports the output note
// 4. On client B imports the note and consumes it
#[test]
fn test_cli_export_import_note() {
    /// This te
    const NOTE_FILENAME: &str = "test_note.mno";

    // For now sleep to ensure node's up
    std::thread::sleep(std::time::Duration::new(30, 0));

    let store_path_1 = create_test_store_path();
    let mut temp_dir_1 = temp_dir();
    temp_dir_1.push(format!("{}", uuid::Uuid::new_v4()));
    dbg!(&temp_dir_1);
    std::fs::create_dir(temp_dir_1.clone()).unwrap();

    let store_path_2 = create_test_store_path();
    let mut temp_dir_2 = temp_dir();
    temp_dir_2.push(format!("{}", uuid::Uuid::new_v4()));
    dbg!(&temp_dir_2);
    std::fs::create_dir(temp_dir_2.clone()).unwrap();

    // Init and create basic wallet on second client
    set_current_dir(temp_dir_2.clone()).unwrap();

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--store-path", store_path_2.to_str().unwrap()]);
    init_cmd.assert().success();

    // Create wallet account
    let mut create_wallet_cmd = Command::cargo_bin("miden").unwrap();
    create_wallet_cmd.args(["new-wallet", "-s", "off-chain"]);
    create_wallet_cmd.assert().success();

    let first_basic_account_id = {
        let client = create_test_client_with_store_path(&store_path_2);
        let accounts = client.get_account_stubs().unwrap();

        accounts.first().unwrap().0.id().to_hex()
    };

    // On first client import the faucet and mint
    set_current_dir(temp_dir_1.clone()).unwrap();

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--store-path", dbg!(&store_path_1.to_str().unwrap())]);
    init_cmd.assert().success();

    // Create faucet account
    let mut create_faucet_cmd = Command::cargo_bin("miden").unwrap();
    create_faucet_cmd.args([
        "new-faucet",
        "-s",
        "off-chain",
        "-t",
        "BTC",
        "-d",
        "8",
        "-m",
        "100000",
    ]);
    create_faucet_cmd.assert().success();

    let fungible_faucet_account_id = {
        let client = create_test_client_with_store_path(&store_path_1);
        let accounts = client.get_account_stubs().unwrap();

        accounts.first().unwrap().0.id().to_hex()
    };

    sync_cli();

    // Let's try and mint
    let mut mint_cmd = Command::cargo_bin("miden").unwrap();
    mint_cmd.args([
        "mint",
        "--target",
        &first_basic_account_id,
        "--faucet",
        &fungible_faucet_account_id,
        "--amount",
        "100",
        "-n",
        "private",
        "--force",
    ]);
    mint_cmd.assert().success();

    // Create a Client to get notes
    let note_to_export_id = {
        let client = create_test_client_with_store_path(&store_path_1);
        let output_notes = client.get_output_notes(NoteFilter::All).unwrap();

        output_notes.first().unwrap().id().to_hex()
    };

    // Export the note
    let mut export_cmd = Command::cargo_bin("miden").unwrap();
    export_cmd.args(["export", &note_to_export_id, "--filename", NOTE_FILENAME]);
    export_cmd.assert().success();

    // Copy the note
    let mut client_1_note_file_path = temp_dir_1.clone();
    client_1_note_file_path.push(NOTE_FILENAME);
    let mut client_2_note_file_path = temp_dir_2.clone();
    client_2_note_file_path.push(NOTE_FILENAME);
    std::fs::copy(client_1_note_file_path, client_2_note_file_path).unwrap();

    // Move to second client to import and consume note
    set_current_dir(temp_dir_2).unwrap();

    // Import Note
    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    import_cmd.args(["import", "--no-verify", NOTE_FILENAME]);
    import_cmd.assert().success();

    // Sleep for a while to ensure the note is committed on the node
    std::thread::sleep(std::time::Duration::new(15, 0));
    sync_cli();

    // Consume the note
    let mut consume_note_cmd = Command::cargo_bin("miden").unwrap();
    consume_note_cmd.args([
        "consume-notes",
        "--account",
        &first_basic_account_id,
        "--force",
        &note_to_export_id,
    ]);
    consume_note_cmd.assert().success();
}

// HELPERS
// ================================================================================================

// Syncs CLI on current directory
fn sync_cli() {
    let mut sync_cmd = Command::cargo_bin("miden").unwrap();
    sync_cmd.args(["sync"]);
    sync_cmd.assert().success();
}

fn create_test_client_with_store_path(store_path: &Path) -> TestClient {
    let client_config = ClientConfig {
        store: store_path.to_str().unwrap().try_into().unwrap(),
        ..Default::default()
    };

    let store = {
        let sqlite_store = SqliteStore::new((&client_config).into()).unwrap();
        Rc::new(sqlite_store)
    };

    let rng = get_random_coin();

    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
    TestClient::new(TonicRpcClient::new(&client_config.rpc), rng, store, authenticator, true)
}
