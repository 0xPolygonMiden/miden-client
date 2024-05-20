use std::env::{set_current_dir, temp_dir};

use assert_cmd::Command;

use crate::create_test_store_path;

// INIT TESTS
// ================================================================================================

#[test]
fn test_init_without_params() {
    // For now sleep to ensure node's up
    std::thread::sleep(std::time::Duration::new(30, 0));

    let temp_dir = temp_dir();
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
    let temp_dir = temp_dir();
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

#[test]
fn test_import_genesis_accounts_can_be_used_for_transactions() {
    let first_basic_account_id = GENESIS_ACCOUNTS_IDS[0];
    let second_basic_account_id = GENESIS_ACCOUNTS_IDS[1];
    let fungible_faucet_account_id = GENESIS_ACCOUNTS_IDS[2];
    // For now sleep to ensure node's up
    std::thread::sleep(std::time::Duration::new(5, 0));

    let store_path = create_test_store_path();
    let temp_dir = temp_dir();

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

    // Sleep for a while to ensure the consumption is done on the node
    std::thread::sleep(std::time::Duration::new(15, 0));
    sync_cli();

    // Consume the note
    let mut consume_note_cmd = Command::cargo_bin("miden").unwrap();
    consume_note_cmd.args(["consume-notes", "--account", first_basic_account_id, "--force"]);
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
    let mut consume_note_cmd = Command::cargo_bin("miden").unwrap();
    consume_note_cmd.args(["consume-notes", "--account", second_basic_account_id, "--force"]);
    consume_note_cmd.assert().success();

    // Sleep for a while to ensure the consumption is done on the node
    std::thread::sleep(std::time::Duration::new(15, 0));
    sync_cli();
}

fn sync_cli() {
    let mut sync_cmd = Command::cargo_bin("miden").unwrap();
    sync_cmd.args(["sync"]);
    sync_cmd.assert().success();
}
