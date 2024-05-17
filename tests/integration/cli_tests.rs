use std::env::{set_current_dir, temp_dir};

use assert_cmd::Command;

use crate::create_test_store_path;

#[test]
fn test_init_without_params() {
    // For now sleep to ensure node's up
    std::thread::sleep(std::time::Duration::new(60, 0));

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
    std::thread::sleep(std::time::Duration::new(60, 0));

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
