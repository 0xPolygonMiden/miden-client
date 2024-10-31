use std::{
    env::{self, temp_dir},
    fs::File,
    io::Read,
    path::Path,
    sync::Arc,
};

use assert_cmd::Command;
use miden_client::{
    accounts::{Account, AccountId, AccountStorageMode, AccountTemplate},
    config::RpcConfig,
    crypto::RpoRandomCoin,
    rpc::TonicRpcClient,
    store::{
        sqlite_store::{config::SqliteStoreConfig, SqliteStore},
        NoteFilter, StoreAuthenticator,
    },
    testing::ACCOUNT_ID_OFF_CHAIN_SENDER,
    transactions::{LocalTransactionProver, ProvingOptions},
    Client, Felt,
};
use rand::Rng;
use uuid::Uuid;

/// CLI TESTS
///
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
    let mut temp_dir = temp_dir();
    temp_dir.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir.clone()).unwrap();

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init"]);
    init_cmd.current_dir(&temp_dir).assert().success();

    sync_cli(&temp_dir);

    // Trying to init twice should result in an error
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init"]);
    init_cmd.current_dir(&temp_dir).assert().failure();
}

#[test]
fn test_init_with_params() {
    let store_path = create_test_store_path();
    let mut temp_dir = temp_dir();
    temp_dir.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir.clone()).unwrap();

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--rpc", "localhost", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir).assert().success();

    // Assert the config file contains the specified contents
    let mut config_path = temp_dir.clone();
    config_path.push("miden-client.toml");
    let mut config_file = File::open(config_path).unwrap();
    let mut config_file_str = String::new();
    config_file.read_to_string(&mut config_file_str).unwrap();

    assert!(config_file_str.contains(store_path.to_str().unwrap()));
    assert!(config_file_str.contains("localhost"));

    sync_cli(&temp_dir);

    // Trying to init twice should result in an error
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--rpc", "localhost", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir).assert().failure();
}

// TX TESTS
// ================================================================================================

/// This test tries to run a mint TX using the CLI for an account that is not tracked.
#[test]
fn test_mint_with_untracked_account() {
    let store_path = create_test_store_path();
    let mut temp_dir = temp_dir();
    temp_dir.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir.clone()).unwrap();

    let target_account_id = {
        let other_store_path = create_test_store_path();
        let mut client = create_test_client_with_store_path(&other_store_path);
        let account_template = AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Private,
        };
        let (account, _seed) = client.new_account(account_template).unwrap();

        account.id().to_hex()
    };

    // On CLI create the faucet and mint
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir).assert().success();

    // Create faucet account
    let mut create_faucet_cmd = Command::cargo_bin("miden").unwrap();
    create_faucet_cmd.args([
        "new-faucet",
        "-s",
        "private",
        "-t",
        "BTC",
        "-d",
        "8",
        "-m",
        "1000000000000",
    ]);
    create_faucet_cmd.current_dir(&temp_dir).assert().success();

    let fungible_faucet_account_id = {
        let client = create_test_client_with_store_path(&store_path);
        let accounts = client.get_account_headers().unwrap();

        accounts.first().unwrap().0.id().to_hex()
    };

    sync_cli(&temp_dir);

    // Let's try and mint
    mint_cli(&temp_dir, &target_account_id, &fungible_faucet_account_id);

    // Sleep for a while to ensure the note is committed on the node
    sync_until_no_notes(&store_path, &temp_dir, NoteFilter::Expected);
}

// IMPORT TESTS
// ================================================================================================

// Only one faucet is being created on the genesis block
const GENESIS_ACCOUNTS_FILENAMES: [&str; 1] = ["faucet.mac"];

// This tests that it's possible to import the genesis accounts and interact with them. To do so it:
//
// 1. Creates a new client
// 2. Imports the genesis account
// 3. Creates a wallet
// 4. Runs a mint tx and syncs until the transaction and note are committed
#[test]
#[ignore = "import genesis test gets ignored by default so integration tests can be ran with dockerized and remote nodes where we might not have the genesis data"]
fn test_import_genesis_accounts_can_be_used_for_transactions() {
    let store_path = create_test_store_path();
    let mut temp_dir = temp_dir();
    temp_dir.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir.clone()).unwrap();

    for genesis_account_filename in GENESIS_ACCOUNTS_FILENAMES {
        let mut new_file_path = temp_dir.clone();
        new_file_path.push(genesis_account_filename);

        let cargo_workspace_dir =
            env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
        let source_path = format!(
            "{}/../../miden-node/accounts/{}",
            cargo_workspace_dir, genesis_account_filename
        );

        std::fs::copy(source_path, new_file_path).unwrap();
    }

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir).assert().success();

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
        let client = create_test_client_with_store_path(&store_path);
        let accounts = client.get_account_headers().unwrap();

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

    // Create wallet account
    let mut create_wallet_cmd = Command::cargo_bin("miden").unwrap();
    create_wallet_cmd.args(["new-wallet", "-s", "private"]);
    create_wallet_cmd.current_dir(&temp_dir).assert().success();

    // Let's try and mint
    mint_cli(
        &temp_dir,
        &AccountId::try_from(ACCOUNT_ID_OFF_CHAIN_SENDER).unwrap().to_hex(),
        &fungible_faucet_account_id,
    );

    // Wait until the note is committed on the node
    sync_until_no_notes(&store_path, &temp_dir, NoteFilter::Expected);
}

// This tests that it's possible to export and import notes into other CLIs. To do so it:
//
// 1. Creates a client A with a faucet
// 2. Creates a client B with a regular account
// 3. On client A runs a mint transaction, and exports the output note
// 4. On client B imports the note and consumes it
#[test]
fn test_cli_export_import_note() {
    const NOTE_FILENAME: &str = "test_note.mno";

    let store_path_1 = create_test_store_path();
    let mut temp_dir_1 = temp_dir();
    temp_dir_1.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir_1.clone()).unwrap();

    let store_path_2 = create_test_store_path();
    let mut temp_dir_2 = temp_dir();
    temp_dir_2.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir_2.clone()).unwrap();

    // Init and create basic wallet on second client
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--store-path", store_path_2.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir_2).assert().success();

    // Create wallet account
    let mut create_wallet_cmd = Command::cargo_bin("miden").unwrap();
    create_wallet_cmd.args(["new-wallet", "-s", "private"]);
    create_wallet_cmd.current_dir(&temp_dir_2).assert().success();

    let first_basic_account_id = {
        let client = create_test_client_with_store_path(&store_path_2);
        let accounts = client.get_account_headers().unwrap();

        accounts.first().unwrap().0.id().to_hex()
    };

    // On first client init, create a faucet and mint
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--store-path", store_path_1.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir_1).assert().success();

    // Create faucet account
    let mut create_faucet_cmd = Command::cargo_bin("miden").unwrap();
    create_faucet_cmd.args([
        "new-faucet",
        "-s",
        "private",
        "-t",
        "BTC",
        "-d",
        "8",
        "-m",
        "100000000000",
    ]);
    create_faucet_cmd.current_dir(&temp_dir_1).assert().success();

    let fungible_faucet_account_id = {
        let client = create_test_client_with_store_path(&store_path_1);
        let accounts = client.get_account_headers().unwrap();

        accounts.first().unwrap().0.id().to_hex()
    };

    sync_cli(&temp_dir_1);

    // Let's try and mint
    mint_cli(&temp_dir_1, &first_basic_account_id, &fungible_faucet_account_id);

    // Create a Client to get notes
    let note_to_export_id = {
        let client = create_test_client_with_store_path(&store_path_1);
        let output_notes = client.get_output_notes(NoteFilter::All).unwrap();

        output_notes.first().unwrap().id().to_hex()
    };

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
    sync_until_no_notes(&store_path_2, &temp_dir_2, NoteFilter::Expected);

    show_note_cli(&temp_dir_2, &note_to_export_id, false);
    // Consume the note
    consume_note_cli(&temp_dir_2, &first_basic_account_id, &[&note_to_export_id]);

    // Test send command
    let mock_target_id: AccountId = AccountId::try_from(ACCOUNT_ID_OFF_CHAIN_SENDER).unwrap();
    send_cli(
        &temp_dir_2,
        &first_basic_account_id,
        &mock_target_id.to_hex(),
        &fungible_faucet_account_id,
    );
}

#[test]
fn test_cli_export_import_account() {
    const ACCOUNT_FILENAME: &str = "test_account.acc";

    let store_path_1 = create_test_store_path();
    let mut temp_dir_1 = temp_dir();
    temp_dir_1.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir_1.clone()).unwrap();

    let store_path_2 = create_test_store_path();
    let mut temp_dir_2 = temp_dir();
    temp_dir_2.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir_2.clone()).unwrap();

    // Init the first client
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--store-path", store_path_1.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir_1).assert().success();

    // Init the second client
    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--store-path", store_path_2.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir_2).assert().success();

    // Create wallet account
    let mut create_wallet_cmd = Command::cargo_bin("miden").unwrap();
    create_wallet_cmd.args(["new-wallet", "-s", "private"]);
    create_wallet_cmd.current_dir(&temp_dir_1).assert().success();

    let first_basic_account_id = {
        let client = create_test_client_with_store_path(&store_path_1);
        let accounts = client.get_account_headers().unwrap();

        accounts.first().unwrap().0.id().to_hex()
    };

    // Export the account
    let mut export_cmd = Command::cargo_bin("miden").unwrap();
    export_cmd.args([
        "export",
        &first_basic_account_id,
        "--account",
        "--filename",
        ACCOUNT_FILENAME,
    ]);
    export_cmd.current_dir(&temp_dir_1).assert().success();

    // Copy the account file
    let mut client_1_account_file_path = temp_dir_1.clone();
    client_1_account_file_path.push(ACCOUNT_FILENAME);
    let mut client_2_account_file_path = temp_dir_2.clone();
    client_2_account_file_path.push(ACCOUNT_FILENAME);
    std::fs::copy(client_1_account_file_path, client_2_account_file_path).unwrap();

    // Import the account from the second client
    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    import_cmd.args(["import", ACCOUNT_FILENAME]);
    import_cmd.current_dir(&temp_dir_2).assert().success();

    // Ensure the account was imported
    let client_2 = create_test_client_with_store_path(&store_path_2);
    assert!(matches!(
        client_2
            .get_account(AccountId::from_hex(&first_basic_account_id).unwrap())
            .unwrap(),
        (Account { .. }, _)
    ));
}

#[test]
fn test_cli_empty_commands() {
    let store_path = create_test_store_path();
    let mut temp_dir = temp_dir();
    temp_dir.push(format!("{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(temp_dir.clone()).unwrap();

    let mut init_cmd = Command::cargo_bin("miden").unwrap();
    init_cmd.args(["init", "--rpc", "localhost", "--store-path", store_path.to_str().unwrap()]);
    init_cmd.current_dir(&temp_dir).assert().success();

    let mut create_faucet_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(create_faucet_cmd.args(["new-faucet"]));

    let mut import_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(import_cmd.args(["export"]));

    let mut mint_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(mint_cmd.args(["mint"]));

    let mut send_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(send_cmd.args(["send"]));

    let mut swam_cmd = Command::cargo_bin("miden").unwrap();
    assert_command_fails_but_does_not_panic(swam_cmd.args(["swap"]));
}

// HELPERS
// ================================================================================================

// Syncs CLI on directory. It'll try syncing until the command executes successfully. If it never
// executes successfully, eventually the test will time out (provided the nextest config has a
// timeout set).
fn sync_cli(cli_path: &Path) {
    loop {
        let mut sync_cmd = Command::cargo_bin("miden").unwrap();
        sync_cmd.args(["sync"]);
        if sync_cmd.current_dir(cli_path).assert().try_success().is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}

/// Mints 100 units of the corresponding faucet using the cli and checks that the command runs
/// successfully given account using the CLI given by `cli_path`.
fn mint_cli(cli_path: &Path, target_account_id: &str, faucet_id: &str) {
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
    mint_cmd.current_dir(cli_path).assert().success();
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

/// Syncs until there are no input notes satisfying the provided filter
fn sync_until_no_notes(store_path: &Path, cli_path: &Path, filter: NoteFilter) {
    let client = create_test_client_with_store_path(store_path);

    while !client.get_input_notes(filter.clone()).unwrap().is_empty() {
        sync_cli(cli_path);
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

pub fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}

pub type TestClient = Client<RpoRandomCoin>;

fn create_test_client_with_store_path(store_path: &Path) -> TestClient {
    let store_config = SqliteStoreConfig::try_from(store_path.to_str().unwrap()).unwrap();
    let rpc_config = RpcConfig::default();

    let store = {
        let sqlite_store = SqliteStore::new(&store_config).unwrap();
        std::sync::Arc::new(sqlite_store)
    };

    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let tx_prover = Arc::new(LocalTransactionProver::new(ProvingOptions::default()));

    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
    TestClient::new(
        Box::new(TonicRpcClient::new(&rpc_config)),
        rng,
        store,
        std::sync::Arc::new(authenticator),
        tx_prover,
        true,
    )
}

fn assert_command_fails_but_does_not_panic(command: &mut Command) {
    let output_error = command.ok().unwrap_err();
    let exit_code = output_error.as_output().unwrap().status.code().unwrap();
    assert_ne!(exit_code, 0); // Command failed
    assert_ne!(exit_code, 101); // Command didn't panic
}
