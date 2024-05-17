use std::env::{set_current_dir, temp_dir};

use assert_cmd::{cargo::CommandCargoExt, Command};

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
    init_cmd.args(&["init"]);
    init_cmd.assert().success();

    let mut sync_cmd = Command::cargo_bin("miden").unwrap();
    sync_cmd.args(&["sync"]);
    sync_cmd.assert().success();
}

#[test]
fn test_init_with_params() {
    // For now sleep to ensure node's up
    std::thread::sleep(std::time::Duration::new(30, 0));

    let store_path = create_test_store_path();
    let temp_dir = temp_dir();
    set_current_dir(temp_dir).unwrap();

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(&["init", "--rpc", "localhost", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.assert().success();

    let mut sync_cmd = Command::cargo_bin("miden").unwrap();
    sync_cmd.args(&["sync"]);
    sync_cmd.assert().success();
}

// IMPORT TESTS
// ================================================================================================

const GENESIS_ACCOUNTS_FILENAMES: [&str; 3] = ["account0.mac", "account1.mac", "account2.mac"];
const GENESIS_ACCOUNTS_IDS: [&str; 3] =
    ["0x8add712899d6ab76", "0x86bac4a17250e9f6", "0xa1834e02152a0f08"];

#[test]
fn test_import_genesis_accounts() {
    // For now sleep to ensure node's up
    std::thread::sleep(std::time::Duration::new(30, 0));

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
    init_cmd.args(&["init", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.assert().success();

    // Import genesis accounts
    let mut args = vec!["import"];
    for filename in GENESIS_ACCOUNTS_FILENAMES {
        args.push(filename);
    }
    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    import_cmd.args(&args);
    import_cmd.assert().success();

    let mut sync_cmd = Command::cargo_bin("miden").unwrap();
    sync_cmd.args(&["sync"]);
    sync_cmd.assert().success();

    // Ensure they've been importing by
    for account_id in GENESIS_ACCOUNTS_IDS {
        let args = vec!["account", "--show", account_id];
        let mut show_cmd = Command::cargo_bin("miden").unwrap();
        show_cmd.args(&args);
        show_cmd.assert().success();
    }
}
