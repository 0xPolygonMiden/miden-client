use std::{collections::BTreeMap, env::temp_dir, time::Duration};

use figment::{
    providers::{Format, Toml},
    Figment,
};
use miden_client::{
    client::{
        accounts::{AccountStorageMode, AccountTemplate},
        get_random_coin,
        rpc::{AccountDetails, NodeRpcClient, TonicRpcClient},
        transactions::transaction_request::{
            PaymentTransactionData, TransactionRequest, TransactionTemplate,
        },
        Client, NoteRelevance,
    },
    config::ClientConfig,
    errors::{ClientError, NodeRpcClientError},
    store::{sqlite_store::SqliteStore, AuthInfo, NoteFilter, TransactionFilter},
};
use miden_objects::{
    accounts::{Account, AccountId, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN},
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset, TokenSymbol},
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{
        Note, NoteAssets, NoteExecutionMode, NoteId, NoteInputs, NoteMetadata, NoteRecipient,
        NoteTag, NoteType,
    },
    transaction::InputNote,
    Felt, Word,
};
use miden_tx::{utils::Serializable, DataStoreError, TransactionExecutorError};
use uuid::Uuid;

pub const ACCOUNT_ID_REGULAR: u64 = ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN;

type TestClient = Client<TonicRpcClient, RpoRandomCoin, SqliteStore>;

const TEST_CLIENT_CONFIG_FILE_PATH: &str = "./tests/config/miden-client.toml";
/// Creates a `TestClient`
///
/// Creates the client using the config at `TEST_CLIENT_CONFIG_FILE_PATH`. The store's path is at a random temporary location, so the store section of the config file is ignored.
///
/// # Panics
///
/// Panics if there is no config file at `TEST_CLIENT_CONFIG_FILE_PATH`, or it cannot be
/// deserialized into a [ClientConfig]
fn create_test_client() -> TestClient {
    let mut client_config: ClientConfig = Figment::from(Toml::file(TEST_CLIENT_CONFIG_FILE_PATH))
        .extract()
        .expect("should be able to read test config at {TEST_CLIENT_CONFIG_FILE_PATH}");

    client_config.store = create_test_store_path()
        .into_os_string()
        .into_string()
        .unwrap()
        .try_into()
        .unwrap();

    let rpc_endpoint = client_config.rpc.endpoint.to_string();
    let store = SqliteStore::new((&client_config).into()).unwrap();
    let executor_store = SqliteStore::new((&client_config).into()).unwrap();
    let rng = get_random_coin();
    TestClient::new(TonicRpcClient::new(&rpc_endpoint), rng, store, executor_store, true)
}

fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}

async fn execute_tx_and_sync(client: &mut TestClient, tx_request: TransactionRequest) {
    println!("Executing transaction...");
    let transaction_execution_result = client.new_transaction(tx_request).unwrap();
    let transaction_id = transaction_execution_result.executed_transaction().id();

    println!("Sending transaction to node");
    client.submit_transaction(transaction_execution_result).await.unwrap();

    // wait until tx is committed
    loop {
        println!("Syncing State...");
        client.sync_state().await.unwrap();

        // Check if executed transaction got committed by the node
        let uncommited_transactions =
            client.get_transactions(TransactionFilter::Uncomitted).unwrap();
        let is_tx_committed = !uncommited_transactions
            .iter()
            .any(|uncommited_tx| uncommited_tx.id == transaction_id);

        if is_tx_committed {
            break;
        }

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
            },
            Err(other_error) => {
                panic!("Unexpected error: {other_error}");
            },
            _ => return,
        }
    }

    panic!("Unable to connect to node");
}

const MINT_AMOUNT: u64 = 1000;
const TRANSFER_AMOUNT: u64 = 59;

/// Sets up a basic client and returns (basic_account, basic_account, faucet_account)
async fn setup(
    client: &mut TestClient,
    accounts_storage_mode: AccountStorageMode,
) -> (Account, Account, Account) {
    // Enusre clean state
    assert!(client.get_account_stubs().unwrap().is_empty());
    assert!(client.get_transactions(TransactionFilter::All).unwrap().is_empty());
    assert!(client.get_input_notes(NoteFilter::All).unwrap().is_empty());

    // Create faucet account
    let (faucet_account, _) = client
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("MATIC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000_000,
            storage_mode: accounts_storage_mode,
        })
        .unwrap();

    // Create regular accounts
    let (first_basic_account, _) = client
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    let (second_basic_account, _) = client
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    println!("Syncing State...");
    client.sync_state().await.unwrap();

    // Get Faucet and regular accounts
    println!("Fetching Accounts...");
    (first_basic_account, second_basic_account, faucet_account)
}

/// Mints a note from faucet_account_id for basic_account_id, waits for inclusion and returns it
async fn mint_note(
    client: &mut TestClient,
    basic_account_id: AccountId,
    faucet_account_id: AccountId,
    note_type: NoteType,
) -> InputNote {
    let (regular_account, _seed) = client.get_account(basic_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 0);

    // Create a Mint Tx for 1000 units of our fungible asset
    let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();
    let tx_template =
        TransactionTemplate::MintFungibleAsset(fungible_asset, basic_account_id, note_type);

    println!("Minting Asset");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    let _ = execute_tx_and_sync(client, tx_request.clone()).await;

    // Check that note is committed and return it
    println!("Fetching Committed Notes...");
    let note_id = tx_request.expected_output_notes()[0].id();
    let note = client.get_input_note(note_id).unwrap();
    note.try_into().unwrap()
}

/// Consumes and wait until the transaction gets committed
/// This assumes the notes contain assets
async fn consume_notes(client: &mut TestClient, account_id: AccountId, input_notes: &[InputNote]) {
    let tx_template =
        TransactionTemplate::ConsumeNotes(account_id, input_notes.iter().map(|n| n.id()).collect());
    println!("Consuming Note...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;
}

async fn assert_account_has_single_asset(
    client: &TestClient,
    account_id: AccountId,
    asset_account_id: AccountId,
    expected_amount: u64,
) {
    let (regular_account, _seed) = client.get_account(account_id).unwrap();

    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.faucet_id(), asset_account_id);
        assert_eq!(fungible_asset.amount(), expected_amount);
    } else {
        panic!("Account has consumed a note and should have a fungible asset");
    }
}

#[tokio::test]
async fn test_onchain_notes_flow() {
    // Client 1 is an offchain faucet which will mint an onchain note for client 2
    let mut client_1 = create_test_client();
    // Client 2 is an offchain account which will consume the note that it will sync from the node
    let mut client_2 = create_test_client();
    // Client 3 will be transferred part of the assets by client 2's account
    let mut client_3 = create_test_client();
    wait_for_node(&mut client_3).await;

    // Create faucet account
    let (faucet_account, _) = client_1
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("MATIC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000_000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    // Create regular accounts
    let (basic_wallet_1, _) = client_2
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    // Create regular accounts
    let (basic_wallet_2, _) = client_3
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();
    client_1.sync_state().await.unwrap();
    client_2.sync_state().await.unwrap();

    let tx_template = TransactionTemplate::MintFungibleAsset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        basic_wallet_1.id(),
        NoteType::Public,
    );

    let tx_request = client_1.build_transaction_request(tx_template).unwrap();
    let note = tx_request.expected_output_notes()[0].clone();
    execute_tx_and_sync(&mut client_1, tx_request).await;

    // Client 2's account should receive the note here:
    client_2.sync_state().await.unwrap();

    // Assert that the note is the same
    let received_note: InputNote = client_2.get_input_note(note.id()).unwrap().try_into().unwrap();
    assert_eq!(received_note.note().authentication_hash(), note.authentication_hash());
    assert_eq!(received_note.note(), &note);

    // consume the note
    consume_notes(&mut client_2, basic_wallet_1.id(), &[received_note]).await;
    assert_account_has_single_asset(
        &client_2,
        basic_wallet_1.id(),
        faucet_account.id(),
        MINT_AMOUNT,
    )
    .await;

    let p2id_asset = FungibleAsset::new(faucet_account.id(), TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(
        PaymentTransactionData::new(p2id_asset.into(), basic_wallet_1.id(), basic_wallet_2.id()),
        NoteType::Public,
    );
    let tx_request = client_2.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client_2, tx_request).await;

    // sync client 3 (basic account 2)
    client_3.sync_state().await.unwrap();
    // client 3 should only have one note
    let note = client_3
        .get_input_notes(NoteFilter::Committed)
        .unwrap()
        .first()
        .unwrap()
        .clone()
        .try_into()
        .unwrap();

    consume_notes(&mut client_3, basic_wallet_2.id(), &[note]).await;
    assert_account_has_single_asset(
        &client_3,
        basic_wallet_2.id(),
        faucet_account.id(),
        TRANSFER_AMOUNT,
    )
    .await;
}

#[tokio::test]
async fn test_added_notes() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let (_, _, faucet_account_stub) = setup(&mut client, AccountStorageMode::Local).await;
    // Mint some asset for an account not tracked by the client. It should not be stored as an
    // input note afterwards since it is not being tracked by the client
    let fungible_asset = FungibleAsset::new(faucet_account_stub.id(), MINT_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::MintFungibleAsset(
        fungible_asset,
        AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap(),
        NoteType::OffChain,
    );
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    println!("Running Mint tx...");
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that no new notes were added
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(notes.is_empty())
}

#[tokio::test]
async fn test_p2id_transfer() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client, AccountStorageMode::Local).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;
    consume_notes(&mut client, from_account_id, &[note]).await;
    assert_account_has_single_asset(&client, from_account_id, faucet_account_id, MINT_AMOUNT).await;

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        NoteType::OffChain,
    );
    println!("Running P2ID tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that note is committed for the second account to consume
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    // Consume P2ID note
    let tx_template = TransactionTemplate::ConsumeNotes(to_account_id, vec![notes[0].id()]);
    println!("Consuming Note...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Ensure we have nothing else to consume
    let current_notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(current_notes.is_empty());

    let (regular_account, seed) = client.get_account(from_account_id).unwrap();

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

    let (regular_account, _seed) = client.get_account(to_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), TRANSFER_AMOUNT);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    assert_note_cannot_be_consumed_twice(&mut client, to_account_id, notes[0].id()).await;
}

#[tokio::test]
async fn test_p2idr_transfer_consumed_by_target() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client, AccountStorageMode::Local).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;
    println!("about to consume");

    consume_notes(&mut client, from_account_id, &[note]).await;
    assert_account_has_single_asset(&client, from_account_id, faucet_account_id, MINT_AMOUNT).await;

    // Do a transfer from first account to second account with Recall. In this situation we'll do
    // the happy path where the `to_account_id` consumes the note
    println!("getting balance");
    let from_account_balance = client
        .get_account(from_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let to_account_balance = client
        .get_account(to_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let current_block_num = client.get_sync_height().unwrap();
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToIdWithRecall(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        current_block_num + 50,
        NoteType::OffChain,
    );
    println!("Running P2IDR tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that note is committed for the second account to consume
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    // Make the `to_account_id` consume P2IDR note
    let tx_template = TransactionTemplate::ConsumeNotes(to_account_id, vec![notes[0].id()]);
    println!("Consuming Note...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    let (regular_account, seed) = client.get_account(from_account_id).unwrap();
    // The seed should not be retrieved due to the account not being new
    assert!(!regular_account.is_new() && seed.is_none());
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    // Validate the transfered amounts
    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), from_account_balance - TRANSFER_AMOUNT);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    let (regular_account, _seed) = client.get_account(to_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), to_account_balance + TRANSFER_AMOUNT);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    assert_note_cannot_be_consumed_twice(&mut client, to_account_id, notes[0].id()).await;
}

#[tokio::test]
async fn test_p2idr_transfer_consumed_by_sender() {
    let mut client = create_test_client();

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client, AccountStorageMode::Local).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;

    consume_notes(&mut client, from_account_id, &[note]).await;
    assert_account_has_single_asset(&client, from_account_id, faucet_account_id, MINT_AMOUNT).await;
    // Do a transfer from first account to second account with Recall. In this situation we'll do
    // the happy path where the `to_account_id` consumes the note
    let from_account_balance = client
        .get_account(from_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let current_block_num = client.get_sync_height().unwrap();
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToIdWithRecall(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        current_block_num + 5,
        NoteType::OffChain,
    );
    println!("Running P2IDR tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that note is committed
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    // Check that it's still too early to consume
    let tx_template = TransactionTemplate::ConsumeNotes(from_account_id, vec![notes[0].id()]);
    println!("Consuming Note (too early)...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    let transaction_execution_result = client.new_transaction(tx_request);
    assert!(transaction_execution_result.is_err_and(|err| {
        matches!(
            err,
            ClientError::TransactionExecutorError(
                TransactionExecutorError::ExecuteTransactionProgramFailed(_)
            )
        )
    }));

    // Wait to consume with the sender account
    println!("Waiting for note to be consumable by sender");
    let current_block_num = client.get_sync_height().unwrap();

    while client.get_sync_height().unwrap() < current_block_num + 5 {
        client.sync_state().await.unwrap();
    }

    // Consume the note with the sender account
    let tx_template = TransactionTemplate::ConsumeNotes(from_account_id, vec![notes[0].id()]);
    println!("Consuming Note...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    let (regular_account, seed) = client.get_account(from_account_id).unwrap();
    // The seed should not be retrieved due to the account not being new
    assert!(!regular_account.is_new() && seed.is_none());
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    // Validate the the sender hasn't lost funds
    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), from_account_balance);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    let (regular_account, _seed) = client.get_account(to_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 0);

    // Check that the target can't consume the note anymore
    assert_note_cannot_be_consumed_twice(&mut client, to_account_id, notes[0].id()).await;
}

async fn assert_note_cannot_be_consumed_twice(
    client: &mut TestClient,
    consuming_account_id: AccountId,
    note_to_consume_id: NoteId,
) {
    // Check that we can't consume the P2ID note again
    let tx_template =
        TransactionTemplate::ConsumeNotes(consuming_account_id, vec![note_to_consume_id]);
    println!("Consuming Note...");

    // Double-spend error expected to be received since we are consuming the same note
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    match client.new_transaction(tx_request) {
        Err(ClientError::TransactionExecutorError(
            TransactionExecutorError::FetchTransactionInputsFailed(
                DataStoreError::NoteAlreadyConsumed(_),
            ),
        )) => {},
        Ok(_) => panic!("Double-spend error: Note should not be consumable!"),
        _ => panic!("Unexpected error: {}", note_to_consume_id.to_hex()),
    }
}

// TODO: We might want to move these functions related to custom transactions to their own module
// file

// CUSTOM TRANSACTION REQUEST
// ================================================================================================
//
// The following functions are for testing custom transaction code. What the test does is:
//
// - Create a custom tx that mints a custom note which checks that the note args are as expected
//   (ie, a word of 4 felts that represent [9, 12, 18, 3])
//
// - Create another transaction that consumes this note with custom code. This custom code only
//   asserts that the {asserted_value} parameter is 0. To test this we first execute with
//   an incorrect value passed in, and after that we try again with the correct value.
//
// Because it's currently not possible to create/consume notes without assets, the P2ID code
// is used as the base for the note code.
#[tokio::test]
async fn test_transaction_request() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let account_template = AccountTemplate::BasicWallet {
        mutable_code: false,
        storage_mode: AccountStorageMode::Local,
    };

    client.sync_state().await.unwrap();
    // Insert Account
    let (regular_account, _seed) = client.new_account(account_template).unwrap();

    let account_template = AccountTemplate::FungibleFaucet {
        token_symbol: TokenSymbol::new("TEST").unwrap(),
        decimals: 5u8,
        max_supply: 10_000u64,
        storage_mode: AccountStorageMode::Local,
    };
    let (fungible_faucet, _seed) = client.new_account(account_template).unwrap();

    // Execute mint transaction in order to create custom note
    let note = mint_custom_note(&mut client, fungible_faucet.id(), regular_account.id()).await;

    client.sync_state().await.unwrap();

    // Prepare transaction

    // If these args were to be modified, the transaction would fail because the note code expects
    // these exact arguments
    let note_args = [[Felt::new(9), Felt::new(12), Felt::new(18), Felt::new(3)]];

    let note_args_map = BTreeMap::from([(note.id(), Some(note_args[0]))]);

    let code = "
        use.miden::contracts::auth::basic->auth_tx
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::memory

        begin
            push.0 push.{asserted_value}
            # => [0, {asserted_value}]
            assert_eq

            call.auth_tx::auth_tx_rpo_falcon512
        end
        ";

    // FAILURE ATTEMPT

    let failure_code = code.replace("{asserted_value}", "1");
    let program = ProgramAst::parse(&failure_code).unwrap();

    let tx_script = {
        let account_auth = client.get_account_auth(regular_account.id()).unwrap();
        let (pubkey_input, advice_map): (Word, Vec<Felt>) = match account_auth {
            AuthInfo::RpoFalcon512(key) => (
                key.public_key().into(),
                key.to_bytes().iter().map(|a| Felt::new(*a as u64)).collect::<Vec<Felt>>(),
            ),
        };

        let script_inputs = vec![(pubkey_input, advice_map)];
        client.compile_tx_script(program, script_inputs, vec![]).unwrap()
    };

    let transaction_request = TransactionRequest::new(
        regular_account.id(),
        note_args_map.clone(),
        vec![],
        Some(tx_script),
    );

    // This fails becuase of {asserted_value} having the incorrect number passed in
    assert!(client.new_transaction(transaction_request).is_err());

    // SUCCESS EXECUTION

    let success_code = code.replace("{asserted_value}", "0");
    let program = ProgramAst::parse(&success_code).unwrap();

    let tx_script = {
        let account_auth = client.get_account_auth(regular_account.id()).unwrap();
        let (pubkey_input, advice_map): (Word, Vec<Felt>) = match account_auth {
            AuthInfo::RpoFalcon512(key) => (
                key.public_key().into(),
                key.to_bytes().iter().map(|a| Felt::new(*a as u64)).collect::<Vec<Felt>>(),
            ),
        };

        let script_inputs = vec![(pubkey_input, advice_map)];
        client.compile_tx_script(program, script_inputs, vec![]).unwrap()
    };

    let transaction_request =
        TransactionRequest::new(regular_account.id(), note_args_map, vec![], Some(tx_script));

    execute_tx_and_sync(&mut client, transaction_request).await;

    client.sync_state().await.unwrap();
}

async fn mint_custom_note(
    client: &mut TestClient,
    faucet_account_id: AccountId,
    target_account_id: AccountId,
) -> Note {
    // Prepare transaction
    let mut random_coin = RpoRandomCoin::new(Default::default());
    let note = create_custom_note(client, faucet_account_id, target_account_id, &mut random_coin);

    let recipient = note
        .recipient_digest()
        .iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".");

    let note_tag = note.metadata().tag().inner();

    let code = "
    use.miden::contracts::faucets::basic_fungible->faucet
    use.miden::contracts::auth::basic->auth_tx
    
    begin
        push.{recipient}
        push.{note_type}
        push.{tag}
        push.{amount}
        call.faucet::distribute
    
        call.auth_tx::auth_tx_rpo_falcon512
        dropw dropw
    end
    "
    .replace("{recipient}", &recipient)
    .replace("{note_type}", &Felt::new(NoteType::OffChain as u64).to_string())
    .replace("{tag}", &Felt::new(note_tag.into()).to_string())
    .replace("{amount}", &Felt::new(10).to_string());

    let program = ProgramAst::parse(&code).unwrap();

    let tx_script = {
        let account_auth = client.get_account_auth(faucet_account_id).unwrap();
        let (pubkey_input, advice_map): (Word, Vec<Felt>) = match account_auth {
            AuthInfo::RpoFalcon512(key) => (
                key.public_key().into(),
                key.to_bytes().iter().map(|a| Felt::new(*a as u64)).collect::<Vec<Felt>>(),
            ),
        };

        let script_inputs = vec![(pubkey_input, advice_map)];
        client.compile_tx_script(program, script_inputs, vec![]).unwrap()
    };

    let transaction_request = TransactionRequest::new(
        faucet_account_id,
        BTreeMap::new(),
        vec![note.clone()],
        Some(tx_script),
    );

    let _ = execute_tx_and_sync(client, transaction_request).await;
    note
}

fn create_custom_note(
    client: &TestClient,
    faucet_account_id: AccountId,
    target_account_id: AccountId,
    rng: &mut RpoRandomCoin,
) -> Note {
    let expected_note_arg = [Felt::new(9), Felt::new(12), Felt::new(18), Felt::new(3)]
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(".");

    let note_script =
        include_str!("asm/custom_p2id.masm").replace("{expected_note_arg}", &expected_note_arg);
    let note_script = ProgramAst::parse(&note_script).unwrap();
    let note_script = client.compile_note_script(note_script, vec![]).unwrap();

    let inputs = NoteInputs::new(vec![target_account_id.into()]).unwrap();
    let serial_num = rng.draw_word();
    let note_metadata = NoteMetadata::new(
        faucet_account_id,
        NoteType::OffChain,
        NoteTag::from_account_id(target_account_id, NoteExecutionMode::Local).unwrap(),
        Default::default(),
    )
    .unwrap();
    let note_assets =
        NoteAssets::new(vec![FungibleAsset::new(faucet_account_id, 10).unwrap().into()]).unwrap();
    let note_recipient = NoteRecipient::new(serial_num, note_script, inputs);
    Note::new(note_assets, note_metadata, note_recipient)
}

#[tokio::test]
async fn test_onchain_accounts() {
    let mut client_1 = create_test_client();
    let mut client_2 = create_test_client();
    wait_for_node(&mut client_2).await;

    let (first_regular_account, _second_regular_account, faucet_account_stub) =
        setup(&mut client_1, AccountStorageMode::OnChain).await;

    let (
        second_client_first_regular_account,
        _other_second_regular_account,
        _other_faucet_account_stub,
    ) = setup(&mut client_2, AccountStorageMode::Local).await;

    let target_account_id = first_regular_account.id();
    let second_client_target_account_id = second_client_first_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    let (_, faucet_seed) = client_1.get_account_stub_by_id(faucet_account_id).unwrap();
    let auth_info = client_1.get_account_auth(faucet_account_id).unwrap();
    client_2.insert_account(&faucet_account_stub, faucet_seed, &auth_info).unwrap();

    // First Mint necesary token
    println!("First client consuming note");
    let note =
        mint_note(&mut client_1, target_account_id, faucet_account_id, NoteType::OffChain).await;

    // Update the state in the other client and ensure the onchain faucet hash is consistent
    // between clients
    client_2.sync_state().await.unwrap();

    let (client_1_faucet, _) = client_1.get_account_stub_by_id(faucet_account_stub.id()).unwrap();
    let (client_2_faucet, _) = client_2.get_account_stub_by_id(faucet_account_stub.id()).unwrap();

    assert_eq!(client_1_faucet.hash(), client_2_faucet.hash());

    // Now use the faucet in the second client to mint to its own account
    println!("Second client consuming note");
    let second_client_note = mint_note(
        &mut client_2,
        second_client_target_account_id,
        faucet_account_id,
        NoteType::OffChain,
    )
    .await;

    // Update the state in the other client and ensure the onchain faucet hash is consistent
    // between clients
    client_1.sync_state().await.unwrap();

    println!("About to consume");
    consume_notes(&mut client_1, target_account_id, &[note]).await;
    assert_account_has_single_asset(&client_1, target_account_id, faucet_account_id, MINT_AMOUNT)
        .await;
    consume_notes(&mut client_2, second_client_target_account_id, &[second_client_note]).await;
    assert_account_has_single_asset(
        &client_2,
        second_client_target_account_id,
        faucet_account_id,
        MINT_AMOUNT,
    )
    .await;

    let (client_1_faucet, _) = client_1.get_account_stub_by_id(faucet_account_stub.id()).unwrap();
    let (client_2_faucet, _) = client_2.get_account_stub_by_id(faucet_account_stub.id()).unwrap();

    assert_eq!(client_1_faucet.hash(), client_2_faucet.hash());

    // Now we'll try to do a p2id transfer from an account of one client to the other one
    let from_account_id = target_account_id;
    let to_account_id = second_client_target_account_id;

    // get initial balances
    let from_account_balance = client_1
        .get_account(from_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let to_account_balance = client_2
        .get_account(to_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);

    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        NoteType::Public,
    );

    println!("Running P2ID tx...");
    let tx_request = client_1.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client_1, tx_request).await;

    // sync on second client until we receive the note
    println!("Syncing on second client...");
    client_2.sync_state().await.unwrap();
    let notes = client_2.get_input_notes(NoteFilter::Committed).unwrap();

    // Consume the note
    println!("Consuming note con second client...");
    let tx_template = TransactionTemplate::ConsumeNotes(to_account_id, vec![notes[0].id()]);
    let tx_request = client_2.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client_2, tx_request).await;

    // sync on first client
    println!("Syncing on first client...");
    client_1.sync_state().await.unwrap();

    let new_from_account_balance = client_1
        .get_account(from_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let new_to_account_balance = client_2
        .get_account(to_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);

    assert_eq!(new_from_account_balance, from_account_balance - TRANSFER_AMOUNT);
    assert_eq!(new_to_account_balance, to_account_balance + TRANSFER_AMOUNT);
}

#[tokio::test]
async fn test_get_consumable_notes() {
    let mut client = create_test_client();

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client, AccountStorageMode::Local).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    //No consumable notes initially
    assert!(client.get_consumable_notes(None).unwrap().is_empty());

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;

    // Check that note is consumable by the account that minted
    assert!(!client.get_consumable_notes(None).unwrap().is_empty());
    assert!(!client.get_consumable_notes(Some(from_account_id)).unwrap().is_empty());
    assert!(client.get_consumable_notes(Some(to_account_id)).unwrap().is_empty());

    consume_notes(&mut client, from_account_id, &[note]).await;

    //After consuming there are no more consumable notes
    assert!(client.get_consumable_notes(None).unwrap().is_empty());

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToIdWithRecall(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        100,
        NoteType::OffChain,
    );
    println!("Running P2IDR tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that note is consumable by both accounts
    let consumable_notes = client.get_consumable_notes(None).unwrap();
    let relevant_accounts = &consumable_notes.first().unwrap().relevances;
    assert_eq!(relevant_accounts.len(), 2);
    assert!(!client.get_consumable_notes(Some(from_account_id)).unwrap().is_empty());
    assert!(!client.get_consumable_notes(Some(to_account_id)).unwrap().is_empty());

    // Check that the note is only consumable after block 100 for the account that sent the transaction
    let from_account_relevance = relevant_accounts
        .iter()
        .find(|relevance| relevance.0 == from_account_id)
        .unwrap()
        .1;
    assert_eq!(from_account_relevance, NoteRelevance::After(100));

    // Check that the note is always consumable for the account that received the transaction
    let to_account_relevance = relevant_accounts
        .iter()
        .find(|relevance| relevance.0 == to_account_id)
        .unwrap()
        .1;
    assert_eq!(to_account_relevance, NoteRelevance::Always);
}

#[tokio::test]
async fn test_get_output_notes() {
    let mut client = create_test_client();

    let (first_regular_account, _, faucet_account_stub) =
        setup(&mut client, AccountStorageMode::Local).await;

    let from_account_id = first_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();
    let random_account_id = AccountId::from_hex("0x0123456789abcdef").unwrap();

    //No output notes initially
    assert!(client.get_output_notes(NoteFilter::All).unwrap().is_empty());

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;

    // Check that there was an output note but it wasn't consumed
    assert!(client.get_output_notes(NoteFilter::Consumed).unwrap().is_empty());
    assert!(!client.get_output_notes(NoteFilter::All).unwrap().is_empty());

    consume_notes(&mut client, from_account_id, &[note]).await;

    //After consuming, the note is returned when using the [NoteFilter::Consumed] filter
    assert!(!client.get_output_notes(NoteFilter::Consumed).unwrap().is_empty());

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, random_account_id),
        NoteType::OffChain,
    );
    println!("Running P2ID tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();

    let output_note_id = tx_request.expected_output_notes()[0].id();

    //Before executing, the output note is not found
    assert!(client.get_output_note(output_note_id).is_err());

    execute_tx_and_sync(&mut client, tx_request).await;

    //After executing, the note is only found in output notes
    assert!(client.get_output_note(output_note_id).is_ok());
    assert!(client.get_input_note(output_note_id).is_err());
}

#[tokio::test]
async fn test_onchain_notes_sync_with_tag() {
    // Client 1 has an offchain faucet which will mint an onchain note for client 2
    let mut client_1 = create_test_client();
    // Client 2 will be used to sync and check that by adding the tag we can still fetch notes
    // whose tag doesn't necessarily match any of its accounts
    let mut client_2 = create_test_client();
    // Client 3 will be the control client. We won't add any tags and expect the note not to be
    // fetched
    let mut client_3 = create_test_client();
    wait_for_node(&mut client_3).await;

    // Create faucet account
    let (faucet_account, _) = client_1
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("MATIC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000_000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    client_1.sync_state().await.unwrap();
    client_2.sync_state().await.unwrap();
    client_3.sync_state().await.unwrap();

    let target_account_id = AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap();
    let tx_template = TransactionTemplate::MintFungibleAsset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        target_account_id,
        NoteType::Public,
    );

    let tx_request = client_1.build_transaction_request(tx_template).unwrap();
    let note = tx_request.expected_output_notes()[0].clone();
    execute_tx_and_sync(&mut client_1, tx_request).await;

    // Load tag into client 2
    client_2
        .add_note_tag(
            NoteTag::from_account_id(target_account_id, NoteExecutionMode::Local).unwrap(),
        )
        .unwrap();

    // Client 2's account should receive the note here:
    client_2.sync_state().await.unwrap();
    client_3.sync_state().await.unwrap();

    // Assert that the note is the same
    let received_note: InputNote = client_2.get_input_note(note.id()).unwrap().try_into().unwrap();
    assert_eq!(received_note.note().authentication_hash(), note.authentication_hash());
    assert_eq!(received_note.note(), &note);
    assert!(client_3.get_input_notes(NoteFilter::All).unwrap().is_empty());
}

#[tokio::test]
async fn test_get_account_update() {
    // Create a client with both public and private accounts.
    let mut client = create_test_client();

    let (basic_wallet_1, _, faucet_account) = setup(&mut client, AccountStorageMode::Local).await;

    let (basic_wallet_2, _) = client
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::OnChain,
        })
        .unwrap();

    // Mint and consume notes with both accounts so they are included in the node.
    let note1 =
        mint_note(&mut client, basic_wallet_1.id(), faucet_account.id(), NoteType::OffChain).await;
    let note2 =
        mint_note(&mut client, basic_wallet_2.id(), faucet_account.id(), NoteType::OffChain).await;

    client.sync_state().await.unwrap();

    consume_notes(&mut client, basic_wallet_1.id(), &[note1]).await;
    consume_notes(&mut client, basic_wallet_2.id(), &[note2]).await;

    wait_for_node(&mut client).await;
    client.sync_state().await.unwrap();

    // Request updates from node for both accounts. The request should not fail and both types of
    // [AccountDetails] should be received.
    let details1 = client.rpc_api().get_account_update(basic_wallet_1.id()).await.unwrap();
    let details2 = client.rpc_api().get_account_update(basic_wallet_2.id()).await.unwrap();

    assert!(matches!(details1, AccountDetails::OffChain(_, _)));
    assert!(matches!(details2, AccountDetails::Public(_, _)));
}
