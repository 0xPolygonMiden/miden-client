use core::panic;
use std::{
    env::{self, temp_dir},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use assert_cmd::Command;
use config::RpcConfig;
use miden_cli::CliKeyStore;
use miden_client::{
    self, Client, Felt,
    account::{AccountId, AccountStorageMode},
    crypto::{FeltRng, RpoRandomCoin},
    note::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteFile, NoteId, NoteInputs,
        NoteMetadata, NoteRecipient, NoteTag, NoteType,
    },
    rpc::{Endpoint, TonicRpcClient},
    store::sqlite_store::SqliteStore,
    testing::account_id::ACCOUNT_ID_PRIVATE_SENDER,
    transaction::{OutputNote, TransactionRequestBuilder},
    utils::Serializable,
};
use miden_client_tests::common::{
    ACCOUNT_ID_REGULAR, TEST_CLIENT_RPC_CONFIG_FILE, execute_tx_and_sync, insert_new_wallet,
};
use predicates::str::contains;
use rand::Rng;
use toml::Table;
use uuid::Uuid;

mod config;

// CLI TESTS
// ================================================================================================

/// This Module contains integration tests that test against the miden CLI directly. In order to do
/// that we use [assert_cmd](https://github.com/assert-rs/assert_cmd?tab=readme-ov-file) which aids
/// in the process of spawning commands.
///
/// Tests added here should only interact with the CLI through `assert_cmd`, with the exception of
/// reading data from the client's store since it would be quite tedious to parse the CLI output
/// for that and is more error prone.
///
/// Note that each client has to run in its own directory so you'll need to create a random
/// temporary directory (check existing tests to see how). You'll also need to make the commands
/// run as if they were spawned on that directory. `std::env::set_current_dir` shouldn't be used as
/// it impacts on other tests and instead you should use `assert_cmd::Command::current_dir`.

// INIT TESTS
// ================================================================================================

#[test]
fn test_init_without_params() {
    let temp_dir = init_cli().1;

    // Trying to init twice should result in an error
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init"]);
    init_cmd.current_dir(&temp_dir).assert().failure();
}

#[test]
fn test_init_with_params() {
    let store_path = create_test_store_path();
    let temp_dir = init_cli_with_store_path("devnet", &store_path);

    // Assert the config file contains the specified contents
    let mut config_path = temp_dir.clone();
    config_path.push("miden-client.toml");
    let mut config_file = File::open(config_path).unwrap();
    let mut config_file_str = String::new();
    config_file.read_to_string(&mut config_file_str).unwrap();

    assert!(config_file_str.contains(store_path.to_str().unwrap()));
    assert!(config_file_str.contains("devnet"));

    // Trying to init twice should result in an error
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--network", "devnet", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir).assert().failure();
}

// TX TESTS
// ================================================================================================

/// This test tries to run a mint TX using the CLI for an account that isn't tracked.
#[tokio::test]
async fn test_mint_with_untracked_account() {
    let temp_dir = init_cli().1;

    // Create faucet account
    let fungible_faucet_account_id = new_faucet_cli(&temp_dir, AccountStorageMode::Private);

    sync_cli(&temp_dir);

    // Let's try and mint
    mint_cli(
        &temp_dir,
        &AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap().to_hex(),
        &fungible_faucet_account_id,
    );

    // Sleep for a while to ensure the note is committed on the node
    sync_until_committed_note(&temp_dir);
}

/// This test tries to run a mint TX using the CLI for an account that isn't tracked.
#[tokio::test]
async fn test_token_symbol_mapping() {
    let (store_path, temp_dir) = init_cli();

    // Create faucet account
    let fungible_faucet_account_id = new_faucet_cli(&temp_dir, AccountStorageMode::Private);

    // Create a token symbol mapping file
    let token_symbol_map_path = temp_dir.join("token_symbol_map.toml");
    let token_symbol_map_content =
        format!(r#"BTC = {{ id = "{fungible_faucet_account_id}", decimals = 10 }}"#,);
    fs::write(&token_symbol_map_path, token_symbol_map_content).unwrap();

    sync_cli(&temp_dir);

    let mut mint_cmd = Command::cargo_bin("miden").unwrap();
    mint_cmd.args([
        "mint",
        "--target",
        AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap().to_hex().as_str(),
        "--asset",
        "0.00001::BTC",
        "-n",
        "private",
        "--force",
    ]);

    let output = mint_cmd.current_dir(&temp_dir).output().unwrap();
    assert!(output.status.success());

    let note_id = String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .skip_while(|&word| word != "Output")
        .find(|word| word.starts_with("0x"))
        .unwrap()
        .to_string();

    let note = {
        let (client, _) = create_rust_client_with_store_path(&store_path).await;
        client
            .get_output_note(NoteId::try_from_hex(&note_id).unwrap())
            .await
            .unwrap()
            .unwrap()
    };

    assert_eq!(note.assets().num_assets(), 1);
    assert_eq!(note.assets().iter().next().unwrap().unwrap_fungible().amount(), 100_000);
}

// IMPORT TESTS
// ================================================================================================

// Only one faucet is being created on the genesis block
const GENESIS_ACCOUNTS_FILENAMES: [&str; 1] = ["account.mac"];

// This tests that it's possible to import the genesis accounts and interact with them. To do so it:
//
// 1. Creates a new client
// 2. Imports the genesis account
// 3. Creates a wallet
// 4. Runs a mint tx and syncs until the transaction and note are committed
#[tokio::test]
#[ignore = "import genesis test gets ignored by default so integration tests can be ran with dockerized and remote nodes where we might not have the genesis data"]
async fn test_import_genesis_accounts_can_be_used_for_transactions() {
    let (store_path, temp_dir) = init_cli();

    for genesis_account_filename in GENESIS_ACCOUNTS_FILENAMES {
        let mut new_file_path = temp_dir.clone();
        new_file_path.push(genesis_account_filename);

        let cargo_workspace_dir =
            env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
        let source_path =
            format!("{cargo_workspace_dir}/../../miden-node/accounts/{genesis_account_filename}",);

        std::fs::copy(source_path, new_file_path).unwrap();
    }

    // Import genesis accounts
    let mut args = vec!["import"];
    for filename in GENESIS_ACCOUNTS_FILENAMES {
        args.push(filename);
    }
    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    import_cmd.args(&args);
    import_cmd.current_dir(&temp_dir).assert().success();

    sync_cli(&temp_dir);

    let fungible_faucet_account_id = {
        let (client, _) = create_rust_client_with_store_path(&store_path).await;
        let accounts = client.get_account_headers().await.unwrap();

        let account_ids = accounts.iter().map(|(acc, _seed)| acc.id()).collect::<Vec<_>>();
        let faucet_accounts = account_ids.iter().filter(|id| id.is_faucet()).collect::<Vec<_>>();

        assert_eq!(faucet_accounts.len(), 1);

        faucet_accounts[0].to_hex()
    };

    // Ensure they've been importing by showing them
    let args = vec!["account", "--show", &fungible_faucet_account_id];
    let mut show_cmd = Command::cargo_bin("miden").unwrap();
    show_cmd.args(&args);
    show_cmd.current_dir(&temp_dir).assert().success();

    // Let's try and mint
    mint_cli(
        &temp_dir,
        &AccountId::try_from(ACCOUNT_ID_PRIVATE_SENDER).unwrap().to_hex(),
        &fungible_faucet_account_id,
    );

    // Wait until the note is committed on the node
    sync_until_committed_note(&temp_dir);
}

// This tests that it's possible to export and import notes into other CLIs. To do so it:
//
// 1. Creates a client A with a faucet
// 2. Creates a client B with a regular account
// 3. On client A runs a mint transaction, and exports the output note
// 4. On client B imports the note and consumes it
#[tokio::test]
async fn test_cli_export_import_note() {
    const NOTE_FILENAME: &str = "test_note.mno";

    let temp_dir_1 = init_cli().1;
    let temp_dir_2 = init_cli().1;

    // Create wallet account
    let first_basic_account_id = new_wallet_cli(&temp_dir_2, AccountStorageMode::Private);

    // Create faucet account
    let fungible_faucet_account_id = new_faucet_cli(&temp_dir_1, AccountStorageMode::Private);

    sync_cli(&temp_dir_1);

    // Let's try and mint
    let note_to_export_id =
        mint_cli(&temp_dir_1, &first_basic_account_id, &fungible_faucet_account_id);

    // Export without type fails
    let mut export_cmd = Command::cargo_bin("miden").unwrap();
    export_cmd.args(["export", &note_to_export_id, "--filename", NOTE_FILENAME]);
    export_cmd.current_dir(&temp_dir_1).assert().failure().code(1); // Code returned when the CLI handles an error

    // Export the note
    let mut export_cmd = Command::cargo_bin("miden").unwrap();
    export_cmd.args([
        "export",
        &note_to_export_id,
        "--filename",
        NOTE_FILENAME,
        "--export-type",
        "partial",
    ]);
    export_cmd.current_dir(&temp_dir_1).assert().success();

    // Copy the note
    let mut client_1_note_file_path = temp_dir_1.clone();
    client_1_note_file_path.push(NOTE_FILENAME);
    let mut client_2_note_file_path = temp_dir_2.clone();
    client_2_note_file_path.push(NOTE_FILENAME);
    std::fs::copy(client_1_note_file_path, client_2_note_file_path).unwrap();

    // Import Note on second client
    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    import_cmd.args(["import", NOTE_FILENAME]);
    import_cmd.current_dir(&temp_dir_2).assert().success();

    // Wait until the note is committed on the node
    sync_until_committed_note(&temp_dir_2);

    show_note_cli(&temp_dir_2, &note_to_export_id, false);
    // Consume the note
    consume_note_cli(&temp_dir_2, &first_basic_account_id, &[&note_to_export_id]);

    // Test send command
    let mock_target_id: AccountId = AccountId::try_from(ACCOUNT_ID_PRIVATE_SENDER).unwrap();
    send_cli(
        &temp_dir_2,
        &first_basic_account_id,
        &mock_target_id.to_hex(),
        &fungible_faucet_account_id,
    );
}

#[tokio::test]
async fn test_cli_export_import_account() {
    const FAUCET_FILENAME: &str = "test_faucet.mac";
    const WALLET_FILENAME: &str = "test_wallet.wal";

    let temp_dir_1 = init_cli().1;
    let (store_path_2, temp_dir_2) = init_cli();

    // Create faucet account
    let faucet_id = new_faucet_cli(&temp_dir_1, AccountStorageMode::Private);

    // Create wallet account
    let wallet_id = new_wallet_cli(&temp_dir_1, AccountStorageMode::Private);

    // Export the accounts
    let mut export_cmd = Command::cargo_bin("miden").unwrap();
    export_cmd.args(["export", &faucet_id, "--account", "--filename", FAUCET_FILENAME]);
    export_cmd.current_dir(&temp_dir_1).assert().success();
    let mut export_cmd = Command::cargo_bin("miden").unwrap();
    export_cmd.args(["export", &wallet_id, "--account", "--filename", WALLET_FILENAME]);
    export_cmd.current_dir(&temp_dir_1).assert().success();

    // Copy the account files
    for filename in &[FAUCET_FILENAME, WALLET_FILENAME] {
        let mut client_1_file_path = temp_dir_1.clone();
        client_1_file_path.push(filename);
        let mut client_2_file_path = temp_dir_2.clone();
        client_2_file_path.push(filename);
        std::fs::copy(client_1_file_path, client_2_file_path).unwrap();
    }

    // Import the account from the second client
    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    import_cmd.args(["import", FAUCET_FILENAME]);
    import_cmd.current_dir(&temp_dir_2).assert().success();
    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    import_cmd.args(["import", WALLET_FILENAME]);
    import_cmd.current_dir(&temp_dir_2).assert().success();

    // Ensure the account was imported
    let client_2 = create_rust_client_with_store_path(&store_path_2).await.0;
    assert!(client_2.get_account(AccountId::from_hex(&faucet_id).unwrap()).await.is_ok());
    assert!(client_2.get_account(AccountId::from_hex(&wallet_id).unwrap()).await.is_ok());

    sync_cli(&temp_dir_2);

    let note_id = mint_cli(&temp_dir_2, &wallet_id, &faucet_id);

    // Wait until the note is committed on the node
    sync_until_committed_note(&temp_dir_2);

    // Consume the note
    consume_note_cli(&temp_dir_2, &wallet_id, &[&note_id]);
}

#[test]
fn test_cli_empty_commands() {
    let temp_dir = init_cli().1;

    let mut create_faucet_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(
        create_faucet_cmd.args(["new-account"]).current_dir(&temp_dir),
    );

    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(import_cmd.args(["export"]).current_dir(&temp_dir));

    let mut mint_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(mint_cmd.args(["mint"]).current_dir(&temp_dir));

    let mut send_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(send_cmd.args(["send"]).current_dir(&temp_dir));

    let mut swam_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(swam_cmd.args(["swap"]).current_dir(&temp_dir));
}

#[tokio::test]
async fn test_consume_unauthenticated_note() {
    let temp_dir = init_cli().1;

    // Create wallet account
    let wallet_account_id = new_wallet_cli(&temp_dir, AccountStorageMode::Public);

    // Create faucet account
    let fungible_faucet_account_id = new_faucet_cli(&temp_dir, AccountStorageMode::Public);

    sync_cli(&temp_dir);

    // Mint
    let note_id = mint_cli(&temp_dir, &wallet_account_id, &fungible_faucet_account_id);

    // Consume the note, internally this checks that the note was consumed correctly
    consume_note_cli(&temp_dir, &wallet_account_id, &[&note_id]);
}

// DEVNET & TESTNET TESTS
// ================================================================================================

#[tokio::test]
async fn test_init_with_devnet() {
    let temp_dir = init_cli_with_store_path("devnet", &create_test_store_path());

    // Check in the config file that the network is devnet
    let mut config_path = temp_dir.clone();
    config_path.push("miden-client.toml");
    let mut config_file = File::open(config_path).unwrap();
    let mut config_file_str = String::new();
    config_file.read_to_string(&mut config_file_str).unwrap();

    assert!(config_file_str.contains(&Endpoint::devnet().to_string()));
}

#[tokio::test]
async fn test_init_with_testnet() {
    let temp_dir = init_cli_with_store_path("testnet", &create_test_store_path());

    // Check in the config file that the network is testnet
    let mut config_path = temp_dir.clone();
    config_path.push("miden-client.toml");
    let mut config_file = File::open(config_path).unwrap();
    let mut config_file_str = String::new();
    config_file.read_to_string(&mut config_file_str).unwrap();

    assert!(config_file_str.contains(&Endpoint::testnet().to_string()));
}

#[tokio::test]
async fn debug_mode_outputs_logs() {
    // This test tries to execute a transaction with debug mode enabled and checks that the stack
    // state is printed. We need to use the CLI for this because the debug logs are always printed
    // to stdout and we can't capture them in a [`Client`] only test.
    // We use the [`Client`] to create a custom note that will print the stack state and consume it
    // using the CLI to check the stdout.

    const NOTE_FILENAME: &str = "test_note.mno";
    unsafe {
        env::set_var("MIDEN_DEBUG", "true");
    }

    // Create a Client and a custom note
    let store_path = create_test_store_path();
    let (mut client, authenticator) = create_rust_client_with_store_path(&store_path).await;
    let (account, ..) = insert_new_wallet(&mut client, AccountStorageMode::Private, &authenticator)
        .await
        .unwrap();

    // Create the custom note with a script that will print the stack state
    let note_script = "
            begin
                debug.stack
                assert_eq
            end
            ";
    let note_script = client.compile_note_script(note_script).unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let serial_num = client.rng().draw_word();
    let note_metadata = NoteMetadata::new(
        account.id(),
        NoteType::Private,
        NoteTag::from_account_id(account.id(), NoteExecutionMode::Local).unwrap(),
        NoteExecutionHint::None,
        Felt::default(),
    )
    .unwrap();
    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note_recipient = NoteRecipient::new(serial_num, note_script, inputs);
    let note = Note::new(note_assets, note_metadata, note_recipient);

    // Send transaction and wait for it to be committed
    client.sync_state().await.unwrap();
    let transaction_request = TransactionRequestBuilder::new()
        .with_own_output_notes(vec![OutputNote::Full(note.clone())])
        .build()
        .unwrap();
    execute_tx_and_sync(&mut client, account.id(), transaction_request).await;

    // Export the note
    let note_file: NoteFile = NoteFile::NoteDetails {
        details: note.clone().into(),
        after_block_num: 0.into(),
        tag: Some(note.metadata().tag()),
    };

    // Import the note into the CLI
    let temp_dir = init_cli_with_store_path(get_network_from_rpc_config().as_str(), &store_path);

    // Serialize the note
    let note_path = temp_dir.join(NOTE_FILENAME);
    let mut file = File::create(note_path.clone()).unwrap();
    file.write_all(&note_file.to_bytes()).unwrap();

    // Import the note
    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    import_cmd.args(["import", note_path.to_str().unwrap()]);
    import_cmd.current_dir(&temp_dir).assert().success();

    sync_cli(&temp_dir);

    // Create wallet account
    let wallet_account_id = new_wallet_cli(&temp_dir, AccountStorageMode::Private);

    // Consume the note and check the output
    let mut consume_note_cmd = Command::cargo_bin("miden").unwrap();
    let note_id = note.id().to_hex();
    let mut cli_args = vec!["consume-notes", "--account", &wallet_account_id[0..8], "--force"];
    cli_args.extend_from_slice(vec![note_id.as_str()].as_slice());
    consume_note_cmd.args(&cli_args);
    consume_note_cmd
        .current_dir(&temp_dir)
        .assert()
        .success()
        .stdout(contains("Stack state"));
}

// HELPERS
// ================================================================================================

/// Derives the network name for the `init` command from the RPC test config file.
fn get_network_from_rpc_config() -> String {
    let rpc_config_toml = TEST_CLIENT_RPC_CONFIG_FILE.parse::<Table>().unwrap();
    let rpc_endpoint_toml = rpc_config_toml["endpoint"].as_table().unwrap();

    let host = rpc_endpoint_toml["host"].as_str().unwrap().to_string();

    if host.contains("devnet") {
        "devnet".to_string()
    } else if host.contains("localhost") {
        "localhost".to_string()
    } else if host.contains("testnet") {
        "testnet".to_string()
    } else {
        panic!("Unknown network")
    }
}

/// Initializes a CLI with the network in the config file and returns the store path and the temp
/// directory where the CLI is running.
fn init_cli() -> (PathBuf, PathBuf) {
    let store_path = create_test_store_path();
    let temp_dir = init_cli_with_store_path(get_network_from_rpc_config().as_str(), &store_path);

    (store_path, temp_dir)
}

/// Initializes a CLI with the given network and store path and returns the temp directory where
/// the CLI is running.
fn init_cli_with_store_path(network: &str, store_path: &Path) -> PathBuf {
    let mut temp_dir = temp_dir();
    temp_dir.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir.clone()).unwrap();

    // Init and create basic wallet on second client
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--network", network, "--store-path", store_path.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir).assert().success();

    temp_dir
}

// Syncs CLI on directory. It'll try syncing until the command executes successfully. If it never
// executes successfully, eventually the test will time out (provided the nextest config has a
// timeout set). It returns the number of updated notes after the sync.
fn sync_cli(cli_path: &Path) -> u64 {
    loop {
        let mut sync_cmd = Command::cargo_bin("miden").unwrap();
        sync_cmd.args(["sync"]);

        let output = sync_cmd.current_dir(cli_path).output().unwrap();

        if output.status.success() {
            let updated_notes = String::from_utf8(output.stdout)
                .unwrap()
                .split_whitespace()
                .skip_while(|&word| word != "notes:")
                .find(|word| word.parse::<u64>().is_ok())
                .unwrap()
                .parse()
                .unwrap();

            return updated_notes;
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}

/// Mints 100 units of the corresponding faucet using the cli and checks that the command runs
/// successfully given account using the CLI given by `cli_path`.
fn mint_cli(cli_path: &Path, target_account_id: &str, faucet_id: &str) -> String {
    let mut mint_cmd = Command::cargo_bin("miden").unwrap();
    mint_cmd.args([
        "mint",
        "--target",
        target_account_id,
        "--asset",
        &format!("100::{faucet_id}"),
        "-n",
        "private",
        "--force",
    ]);

    let output = mint_cmd.current_dir(cli_path).output().unwrap();
    assert!(output.status.success());

    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .skip_while(|&word| word != "Output")
        .find(|word| word.starts_with("0x"))
        .unwrap()
        .to_string()
}

/// Shows note details using the cli and checks that the command runs
/// successfully given account using the CLI given by `cli_path`.
fn show_note_cli(cli_path: &Path, note_id: &str, should_fail: bool) {
    let mut show_note_cmd = Command::cargo_bin("miden").unwrap();
    show_note_cmd.args(["notes", "--show", note_id]);

    if should_fail {
        show_note_cmd.current_dir(cli_path).assert().failure();
    } else {
        show_note_cmd.current_dir(cli_path).assert().success();
    }
}

/// Sends 25 units of the corresponding faucet and checks that the command runs successfully given
/// account using the CLI given by `cli_path`.
fn send_cli(cli_path: &Path, from_account_id: &str, to_account_id: &str, faucet_id: &str) {
    let mut send_cmd = Command::cargo_bin("miden").unwrap();
    send_cmd.args([
        "send",
        "--sender",
        &from_account_id[0..8],
        "--target",
        to_account_id,
        "--asset",
        &format!("25::{faucet_id}"),
        "-n",
        "private",
        "--force",
    ]);
    send_cmd.current_dir(cli_path).assert().success();
}

/// Syncs until a tracked note gets committed.
fn sync_until_committed_note(cli_path: &Path) {
    while sync_cli(cli_path) == 0 {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

/// Consumes a series of notes with a given account using the CLI given by `cli_path`.
fn consume_note_cli(cli_path: &Path, account_id: &str, note_ids: &[&str]) {
    let mut consume_note_cmd = Command::cargo_bin("miden").unwrap();
    let mut cli_args = vec!["consume-notes", "--account", &account_id[0..8], "--force"];
    cli_args.extend_from_slice(note_ids);
    consume_note_cmd.args(&cli_args);
    consume_note_cmd.current_dir(cli_path).assert().success();
}

/// Creates a new faucet account using the CLI given by `cli_path`.
fn new_faucet_cli(cli_path: &Path, storage_mode: AccountStorageMode) -> String {
    const INIT_DATA_FILENAME: &str = "init_data.toml";
    let mut create_faucet_cmd = Command::cargo_bin("miden").unwrap();

    // Create a TOML file with the InitStorageData
    let init_storage_data_toml = r#"
        token_metadata.decimals=10
        token_metadata.max_supply=10000000
        token_metadata.ticker="BTC"
        "#;
    let file_path = cli_path.join(INIT_DATA_FILENAME);
    fs::write(&file_path, init_storage_data_toml).unwrap();

    create_faucet_cmd.args([
        "new-account",
        "-s",
        storage_mode.to_string().as_str(),
        "--account-type",
        "fungible-faucet",
        "-c",
        "basic-fungible-faucet",
        "-i",
        INIT_DATA_FILENAME,
    ]);
    create_faucet_cmd.current_dir(cli_path).assert().success();

    let output = create_faucet_cmd.current_dir(cli_path).output().unwrap();
    assert!(output.status.success());

    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .find(|word| word.starts_with("0x"))
        .unwrap()
        .trim_end_matches(|c: char| !c.is_alphanumeric())
        .to_string()
}

/// Creates a new wallet account using the CLI given by `cli_path`.
fn new_wallet_cli(cli_path: &Path, storage_mode: AccountStorageMode) -> String {
    let mut create_wallet_cmd = Command::cargo_bin("miden").unwrap();
    create_wallet_cmd.args(["new-wallet", "-s", storage_mode.to_string().as_str()]);

    let output = create_wallet_cmd.current_dir(cli_path).output().unwrap();
    assert!(output.status.success());

    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .find(|word| word.starts_with("0x"))
        .unwrap()
        .trim_end_matches(|c: char| !c.is_alphanumeric())
        .to_string()
}

/// Creates a temporary sqlite store file.
pub fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}

pub type TestClient = Client;

/// Creates a new [`Client`] with a given store. Also returns the keystore associated with it.
async fn create_rust_client_with_store_path(store_path: &Path) -> (TestClient, CliKeyStore) {
    let rpc_config_toml = TEST_CLIENT_RPC_CONFIG_FILE.parse::<Table>().unwrap();
    let rpc_endpoint_toml = rpc_config_toml["endpoint"].as_table().unwrap();

    let protocol = rpc_endpoint_toml["protocol"].as_str().unwrap().to_string();
    let host = rpc_endpoint_toml["host"].as_str().unwrap().to_string();
    let port = if rpc_endpoint_toml.contains_key("port") {
        rpc_endpoint_toml["port"].as_integer().map(|port| u16::try_from(port).unwrap())
    } else {
        None
    };

    let endpoint = Endpoint::new(protocol, host, port);

    let store = {
        let sqlite_store = SqliteStore::new(PathBuf::from(store_path)).await.unwrap();
        std::sync::Arc::new(sqlite_store)
    };

    let mut rng = rand::rng();
    let coin_seed: [u64; 4] = rng.random();

    let rng = Box::new(RpoRandomCoin::new(coin_seed.map(Felt::new)));

    let keystore = CliKeyStore::new(temp_dir()).unwrap();

    (
        TestClient::new(
            Arc::new(TonicRpcClient::new(&endpoint, RpcConfig::default().timeout_ms)),
            rng,
            store,
            std::sync::Arc::new(keystore.clone()),
            true,
            None,
        ),
        keystore,
    )
}

/// Executes a command and asserts that it fails but does not panic.
fn assert_command_fails_but_does_not_panic(command: &mut Command) {
    let output_error = command.ok().unwrap_err();
    let exit_code = output_error.as_output().unwrap().status.code().unwrap();
    assert_ne!(exit_code, 0); // Command failed
    assert_ne!(exit_code, 101); // Command didn't panic
}

// COMMANDS TESTS
// ================================================================================================

#[test]
fn test_exec_parse() {
    let failure_script =
        fs::canonicalize("tests/files/test_cli_advice_inputs_expect_failure.masm").unwrap();
    let success_script =
        fs::canonicalize("tests/files/test_cli_advice_inputs_expect_success.masm").unwrap();
    let toml_path = fs::canonicalize("tests/files/test_cli_advice_inputs_input.toml").unwrap();

    let temp_dir = init_cli().1;

    // Create wallet account
    let basic_account_id = new_wallet_cli(&temp_dir, AccountStorageMode::Private);

    let mut success_cmd = Command::cargo_bin("miden").unwrap();
    success_cmd.args([
        "exec",
        "-s",
        success_script.to_str().unwrap(),
        "-a",
        &basic_account_id[0..8],
        "-i",
        toml_path.to_str().unwrap(),
    ]);

    success_cmd.current_dir(&temp_dir).assert().success();

    let mut failure_cmd = Command::cargo_bin("miden").unwrap();
    failure_cmd.args([
        "exec",
        "-s",
        failure_script.to_str().unwrap(),
        "-a",
        &basic_account_id[0..8],
        "-i",
        toml_path.to_str().unwrap(),
    ]);

    failure_cmd.current_dir(&temp_dir).assert().failure();
}
