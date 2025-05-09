use std::{
    env::temp_dir,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use miden_client::{
    Client, ClientError, Word,
    account::{
        AccountBuilder, AccountType,
        component::{BasicFungibleFaucet, BasicWallet, RpoFalcon512},
    },
    auth::AuthSecretKey,
    crypto::FeltRng,
    keystore::FilesystemKeyStore,
    note::{Note, create_p2id_note},
    rpc::{Endpoint, RpcError, TonicRpcClient},
    store::{NoteFilter, TransactionFilter, sqlite_store::SqliteStore},
    sync::SyncSummary,
    testing::account_id::ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE,
    transaction::{
        NoteArgs, TransactionRequest, TransactionRequestBuilder, TransactionRequestError,
        TransactionStatus,
    },
};
use miden_objects::{
    Felt, FieldElement,
    account::{Account, AccountId, AccountStorageMode},
    asset::{Asset, FungibleAsset, TokenSymbol},
    crypto::{dsa::rpo_falcon512::SecretKey, rand::RpoRandomCoin},
    note::{NoteId, NoteType},
    transaction::{InputNote, OutputNote, TransactionId},
};
use rand::{Rng, RngCore, rngs::StdRng};
use toml::Table;
use uuid::Uuid;

pub const ACCOUNT_ID_REGULAR: u128 = ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE;

pub type TestClient = Client;
pub type TestClientKeyStore = FilesystemKeyStore<StdRng>;

pub const TEST_CLIENT_RPC_CONFIG_FILE: &str = include_str!("../config/miden-client-rpc.toml");
/// Creates a `TestClient`.
///
/// Creates the client using the config at `TEST_CLIENT_CONFIG_FILE_PATH`. The store's path is at a
/// random temporary location, so the store section of the config file is ignored.
///
/// # Panics
///
/// Panics if there is no config file at `TEST_CLIENT_CONFIG_FILE_PATH`, or it cannot be
/// deserialized into a [ClientConfig].
pub async fn create_test_client() -> (TestClient, TestClientKeyStore) {
    let (rpc_endpoint, rpc_timeout, store_config, auth_path) = get_client_config();

    let store = {
        let sqlite_store = SqliteStore::new(store_config).await.unwrap();
        std::sync::Arc::new(sqlite_store)
    };

    let mut rng = rand::rng();
    let coin_seed: [u64; 4] = rng.random();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));

    let keystore = FilesystemKeyStore::new(auth_path).unwrap();

    let mut client = TestClient::new(
        Arc::new(TonicRpcClient::new(&rpc_endpoint, rpc_timeout)),
        Box::new(rng),
        store,
        Arc::new(keystore.clone()),
        true,
        None,
    );

    client.sync_state().await.unwrap();

    (client, keystore)
}

pub fn get_client_config() -> (Endpoint, u64, PathBuf, PathBuf) {
    let rpc_config_toml = TEST_CLIENT_RPC_CONFIG_FILE.parse::<Table>().unwrap();
    let rpc_endpoint_toml = rpc_config_toml["endpoint"].as_table().unwrap();

    let protocol = rpc_endpoint_toml["protocol"].as_str().unwrap().to_string();
    let host = rpc_endpoint_toml["host"].as_str().unwrap().to_string();
    let port = if rpc_endpoint_toml.contains_key("port") {
        rpc_endpoint_toml["port"].as_integer().map(|port| port as u16)
    } else {
        None
    };

    let endpoint = Endpoint::new(protocol, host, port);

    let timeout_ms = rpc_config_toml["timeout"].as_integer().unwrap() as u64;

    let auth_path = temp_dir().join(format!("keystore-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&auth_path).unwrap();

    (endpoint, timeout_ms, create_test_store_path(), auth_path)
}

pub fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}

pub async fn insert_new_wallet(
    client: &mut Client,
    storage_mode: AccountStorageMode,
    keystore: &TestClientKeyStore,
) -> Result<(Account, Word, SecretKey), ClientError> {
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    insert_new_wallet_with_seed(client, storage_mode, keystore, init_seed).await
}

pub async fn insert_new_wallet_with_seed(
    client: &mut Client,
    storage_mode: AccountStorageMode,
    keystore: &TestClientKeyStore,
    init_seed: [u8; 32],
) -> Result<(Account, Word, SecretKey), ClientError> {
    let key_pair = SecretKey::with_rng(client.rng());
    let pub_key = key_pair.public_key();

    keystore.add_key(&AuthSecretKey::RpoFalcon512(key_pair.clone())).unwrap();

    let anchor_block = client.get_latest_epoch_block().await.unwrap();

    let (account, seed) = AccountBuilder::new(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(storage_mode)
        .with_component(RpoFalcon512::new(pub_key))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    client.add_account(&account, Some(seed), false).await?;

    Ok((account, seed, key_pair))
}

pub async fn insert_new_fungible_faucet(
    client: &mut Client,
    storage_mode: AccountStorageMode,
    keystore: &TestClientKeyStore,
) -> Result<(Account, Word, SecretKey), ClientError> {
    let key_pair = SecretKey::with_rng(client.rng());
    let pub_key = key_pair.public_key();

    keystore.add_key(&AuthSecretKey::RpoFalcon512(key_pair.clone())).unwrap();

    // we need to use an initial seed to create the wallet account
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let symbol = TokenSymbol::new("TEST").unwrap();
    let max_supply = Felt::try_from(9999999_u64.to_le_bytes().as_slice())
        .expect("u64 can be safely converted to a field element");

    let anchor_block = client.get_latest_epoch_block().await.unwrap();

    let (account, seed) = AccountBuilder::new(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(AccountType::FungibleFaucet)
        .storage_mode(storage_mode)
        .with_component(RpoFalcon512::new(pub_key))
        .with_component(BasicFungibleFaucet::new(symbol, 10, max_supply).unwrap())
        .build()
        .unwrap();

    client.add_account(&account, Some(seed), false).await?;
    Ok((account, seed, key_pair))
}

pub async fn execute_failing_tx(
    client: &mut TestClient,
    account_id: AccountId,
    tx_request: TransactionRequest,
    expected_error: ClientError,
) {
    println!("Executing transaction...");
    // We compare string since we can't compare the error directly
    assert_eq!(
        client.new_transaction(account_id, tx_request).await.unwrap_err().to_string(),
        expected_error.to_string()
    );
}

pub async fn execute_tx(
    client: &mut TestClient,
    account_id: AccountId,
    tx_request: TransactionRequest,
) -> TransactionId {
    println!("Executing transaction...");
    let transaction_execution_result =
        client.new_transaction(account_id, tx_request).await.unwrap();
    let transaction_id = transaction_execution_result.executed_transaction().id();

    println!("Sending transaction to node");
    client.submit_transaction(transaction_execution_result).await.unwrap();

    transaction_id
}

pub async fn execute_tx_and_sync(
    client: &mut TestClient,
    account_id: AccountId,
    tx_request: TransactionRequest,
) {
    let transaction_id = execute_tx(client, account_id, tx_request).await;
    wait_for_tx(client, transaction_id).await;
}

pub async fn wait_for_tx(client: &mut TestClient, transaction_id: TransactionId) {
    // wait until tx is committed
    let now = Instant::now();
    loop {
        client.sync_state().await.unwrap();

        // Check if executed transaction got committed by the node
        let tracked_transaction = client
            .get_transactions(TransactionFilter::Ids(vec![transaction_id]))
            .await
            .unwrap()
            .pop()
            .unwrap();
        let is_tx_committed =
            matches!(tracked_transaction.transaction_status, TransactionStatus::Committed(_));

        if is_tx_committed {
            break;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    // Log wait time in a file if the env var is set
    // This allows us to aggregate and measure how long the tests are waiting for transactions to be
    // committed
    if std::env::var("LOG_WAIT_TIMES") == Ok("true".to_string()) {
        let elapsed = now.elapsed();
        let wait_times_dir = std::path::PathBuf::from("wait_times");
        std::fs::create_dir_all(&wait_times_dir).unwrap();

        let elapsed_time_file = wait_times_dir.join(format!("wait_time_{}", Uuid::new_v4()));
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(elapsed_time_file)
            .unwrap();
        writeln!(file, "{:?}", elapsed.as_millis()).unwrap();
    }
}

// Syncs until `amount_of_blocks` have been created onchain compared to client's sync height
pub async fn wait_for_blocks(client: &mut TestClient, amount_of_blocks: u32) -> SyncSummary {
    let current_block = client.get_sync_height().await.unwrap();
    let final_block = current_block + amount_of_blocks;
    println!("Syncing until block {}...", final_block);
    loop {
        let summary = client.sync_state().await.unwrap();
        println!("Synced to block {} (syncing until {})...", summary.block_num, final_block);

        if summary.block_num >= final_block {
            return summary;
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Waits for node to be running.
///
/// # Panics
///
/// This function will panic if it does `NUMBER_OF_NODE_ATTEMPTS` unsuccessful checks or if we
/// receive an error other than a connection related error.
pub async fn wait_for_node(client: &mut TestClient) {
    const NODE_TIME_BETWEEN_ATTEMPTS: u64 = 5;
    const NUMBER_OF_NODE_ATTEMPTS: u64 = 60;

    println!(
        "Waiting for Node to be up. Checking every {NODE_TIME_BETWEEN_ATTEMPTS}s for {NUMBER_OF_NODE_ATTEMPTS} tries..."
    );

    for _try_number in 0..NUMBER_OF_NODE_ATTEMPTS {
        match client.sync_state().await {
            Err(ClientError::RpcError(RpcError::ConnectionError(_))) => {
                std::thread::sleep(Duration::from_secs(NODE_TIME_BETWEEN_ATTEMPTS));
            },
            Err(other_error) => {
                panic!("Unexpected error: {other_error}");
            },
            _ => return,
        }
    }

    panic!("Unable to connect to node");
}

pub const MINT_AMOUNT: u64 = 1000;
pub const TRANSFER_AMOUNT: u64 = 59;

/// Sets up a basic client and returns (basic_account, basic_account, faucet_account).
pub async fn setup_two_wallets_and_faucet(
    client: &mut TestClient,
    accounts_storage_mode: AccountStorageMode,
    keystore: &TestClientKeyStore,
) -> (Account, Account, Account) {
    // Enusre clean state
    assert!(client.get_account_headers().await.unwrap().is_empty());
    assert!(client.get_transactions(TransactionFilter::All).await.unwrap().is_empty());
    assert!(client.get_input_notes(NoteFilter::All).await.unwrap().is_empty());

    // Create faucet account
    let (faucet_account, ..) = insert_new_fungible_faucet(client, accounts_storage_mode, keystore)
        .await
        .unwrap();

    // Create regular accounts
    let (first_basic_account, ..) =
        insert_new_wallet(client, accounts_storage_mode, keystore).await.unwrap();

    let (second_basic_account, ..) =
        insert_new_wallet(client, accounts_storage_mode, keystore).await.unwrap();

    println!("Syncing State...");
    client.sync_state().await.unwrap();

    // Get Faucet and regular accounts
    println!("Fetching Accounts...");
    (first_basic_account, second_basic_account, faucet_account)
}

/// Sets up a basic client and returns (basic_account, faucet_account).
pub async fn setup_wallet_and_faucet(
    client: &mut TestClient,
    accounts_storage_mode: AccountStorageMode,
    keystore: &TestClientKeyStore,
) -> (Account, Account) {
    // Enusre clean state
    assert!(client.get_account_headers().await.unwrap().is_empty());
    assert!(client.get_transactions(TransactionFilter::All).await.unwrap().is_empty());
    assert!(client.get_input_notes(NoteFilter::All).await.unwrap().is_empty());

    let (faucet_account, ..) = insert_new_fungible_faucet(client, accounts_storage_mode, keystore)
        .await
        .unwrap();

    let (basic_account, ..) =
        insert_new_wallet(client, accounts_storage_mode, keystore).await.unwrap();

    (basic_account, faucet_account)
}

/// Mints a note from faucet_account_id for basic_account_id, waits for inclusion and returns it
/// with 1000 units of the corresponding fungible asset.
pub async fn mint_note(
    client: &mut TestClient,
    basic_account_id: AccountId,
    faucet_account_id: AccountId,
    note_type: NoteType,
) -> InputNote {
    // Create a Mint Tx for 1000 units of our fungible asset
    let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();
    println!("Minting Asset");
    let tx_request = TransactionRequestBuilder::mint_fungible_asset(
        fungible_asset,
        basic_account_id,
        note_type,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    execute_tx_and_sync(client, fungible_asset.faucet_id(), tx_request.clone()).await;

    // Check that note is committed and return it
    println!("Fetching Committed Notes...");
    let note_id = tx_request.expected_output_notes().next().unwrap().id();
    let note = client.get_input_note(note_id).await.unwrap().unwrap();
    note.try_into().unwrap()
}

/// Consumes and wait until the transaction gets committed.
/// This assumes the notes contain assets.
pub async fn consume_notes(
    client: &mut TestClient,
    account_id: AccountId,
    input_notes: &[InputNote],
) {
    println!("Consuming Note...");
    let tx_request =
        TransactionRequestBuilder::consume_notes(input_notes.iter().map(|n| n.id()).collect())
            .build()
            .unwrap();
    execute_tx_and_sync(client, account_id, tx_request).await;
}

/// Asserts that the account has a single asset with the expected amount.
pub async fn assert_account_has_single_asset(
    client: &TestClient,
    account_id: AccountId,
    asset_account_id: AccountId,
    expected_amount: u64,
) {
    let regular_account: Account = client.get_account(account_id).await.unwrap().unwrap().into();

    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.faucet_id(), asset_account_id);
        assert_eq!(fungible_asset.amount(), expected_amount);
    } else {
        panic!("Account has consumed a note and should have a fungible asset");
    }
}

/// Tries to consume the note and asserts that the expected error is returned.
pub async fn assert_note_cannot_be_consumed_twice(
    client: &mut TestClient,
    consuming_account_id: AccountId,
    note_to_consume_id: NoteId,
) {
    // Check that we can't consume the P2ID note again
    println!("Consuming Note...");

    // Double-spend error expected to be received since we are consuming the same note
    let tx_request = TransactionRequestBuilder::consume_notes(vec![note_to_consume_id])
        .build()
        .unwrap();

    match client.new_transaction(consuming_account_id, tx_request).await {
        Err(ClientError::TransactionRequestError(
            TransactionRequestError::InputNoteAlreadyConsumed(_),
        )) => {},
        Ok(_) => panic!("Double-spend error: Note should not be consumable!"),
        err => panic!("Unexpected error {:?} for note ID: {}", err, note_to_consume_id.to_hex()),
    }
}

/// Creates a transaction request that mint assets for each target_id account.
pub fn mint_multiple_fungible_asset(
    asset: FungibleAsset,
    target_id: Vec<AccountId>,
    note_type: NoteType,
    rng: &mut impl FeltRng,
) -> TransactionRequest {
    let notes = target_id
        .iter()
        .map(|account_id| {
            OutputNote::Full(
                create_p2id_note(
                    asset.faucet_id(),
                    *account_id,
                    vec![asset.into()],
                    note_type,
                    Felt::ZERO,
                    rng,
                )
                .unwrap(),
            )
        })
        .collect::<Vec<OutputNote>>();

    TransactionRequestBuilder::new().with_own_output_notes(notes).build().unwrap()
}

/// Executes a transaction and consumes the resulting unauthenticated notes inmediately without
/// waiting for the first transaction to be committed.
pub async fn execute_tx_and_consume_output_notes(
    tx_request: TransactionRequest,
    client: &mut TestClient,
    executor: AccountId,
    consumer: AccountId,
) {
    let output_notes = tx_request
        .expected_output_notes()
        .cloned()
        .map(|note| (note, None::<NoteArgs>))
        .collect::<Vec<(Note, Option<NoteArgs>)>>();

    execute_tx(client, executor, tx_request).await;

    let tx_request = TransactionRequestBuilder::new()
        .with_unauthenticated_input_notes(output_notes)
        .build()
        .unwrap();
    let transaction_id = execute_tx(client, consumer, tx_request).await;
    wait_for_tx(client, transaction_id).await;
}

/// Mint assets for the target account and consume them inmediately without waiting for the first
/// transaction to be committed.
pub async fn mint_and_consume(
    client: &mut TestClient,
    basic_account_id: AccountId,
    faucet_account_id: AccountId,
    note_type: NoteType,
) {
    let tx_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap(),
        basic_account_id,
        note_type,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    execute_tx_and_consume_output_notes(tx_request, client, faucet_account_id, basic_account_id)
        .await;
}
