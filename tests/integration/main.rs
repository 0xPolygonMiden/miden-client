use std::{collections::BTreeMap, env::temp_dir, fs, time::Duration};

use miden_client::{
    client::{
        accounts::{AccountStorageMode, AccountTemplate},
        rpc::TonicRpcClient,
        transactions::{
            transaction_request::{
                PaymentTransactionData, TransactionRequest, TransactionTemplate,
            },
            TransactionResult,
        },
        Client,
    },
    config::{ClientConfig, RpcConfig},
    errors::{ClientError, NodeRpcClientError},
    store::{sqlite_store::SqliteStore, AuthInfo, InputNoteRecord, NoteFilter, TransactionFilter},
};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{AccountId, AccountStub},
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset, TokenSymbol},
    crypto::rand::{FeltRng, RpoRandomCoin},
    notes::{Note, NoteId, NoteScript},
    Felt, Word,
};
use miden_tx::{utils::Serializable, DataStoreError, TransactionExecutorError};
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
    let executor_store = SqliteStore::new((&client_config).into()).unwrap();
    TestClient::new(TonicRpcClient::new(&rpc_endpoint), store, executor_store).unwrap()
}

fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}

async fn execute_tx_and_sync(
    client: &mut TestClient,
    tx_request: TransactionRequest,
) {
    println!("Executing Transaction");
    let transaction_execution_result = client.new_transaction(tx_request).unwrap();
    let transaction_id = transaction_execution_result.executed_transaction().id();

    println!("Sending Transaction to node");
    client.send_transaction(transaction_execution_result).await.unwrap();

    // wait until tx is committed
    loop {
        println!("Syncing State...");
        client.sync_state().await.unwrap();

        // Check if executed transaction got committed by the node
        let uncommited_transactions =
            client.get_transactions(TransactionFilter::Uncomitted).unwrap();
        let is_tx_committed = uncommited_transactions
            .iter()
            .find(|uncommited_tx| uncommited_tx.id == transaction_id)
            .is_none();
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
const TRANSFER_AMOUNT: u64 = 50;

#[tokio::test]
async fn main() {
    // TODO: once issue 201
    // (https://github.com/0xPolygonMiden/miden-client/issues/201#issuecomment-1989432215)
    // gets fixed, we should set #[tokio::test] for the following functions individually
    // and delete main() so tests are run in parallel and as individual tests

    test_p2id_transfer().await;
    test_p2idr_transfer().await;
    test_transaction_request().await;

    println!("Test ran successfully!");
}

async fn setup(client: &mut TestClient) -> (AccountStub, AccountStub, AccountStub) {
    // Enusre clean state
    assert!(client.get_accounts().unwrap().is_empty());
    assert!(client.get_transactions(TransactionFilter::All).unwrap().is_empty());
    assert!(client.get_input_notes(NoteFilter::All).unwrap().is_empty());

    // Create faucet account
    client
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("MATIC").unwrap(),
            decimals: 8,
            max_supply: 1000000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    // Create regular accounts
    for _ in 0..2 {
        client
            .new_account(AccountTemplate::BasicWallet {
                mutable_code: false,
                storage_mode: AccountStorageMode::Local,
            })
            .unwrap();
    }

    wait_for_node(client).await;

    println!("Syncing State...");
    client.sync_state().await.unwrap();

    // Get Faucet and regular accounts
    println!("Fetching Accounts...");
    let accounts = client.get_accounts().unwrap();
    assert_eq!(accounts.len(), 3);
    let regular_account_stubs = accounts
        .iter()
        .filter(|(account, _seed)| account.id().is_regular_account())
        .map(|(account, _seed)| account.clone())
        .collect::<Vec<_>>();
    let (faucet_account_stub, _seed) = accounts
        .into_iter()
        .find(|(account, _seed)| !account.id().is_regular_account())
        .unwrap();

    (
        regular_account_stubs[0].clone(),
        regular_account_stubs[1].clone(),
        faucet_account_stub,
    )
}

async fn mint_note(
    client: &mut TestClient,
    first_regular_account_id: AccountId,
    faucet_account_id: AccountId,
) {
    let (regular_account, _seed) = client.get_account(first_regular_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 0);

    // Create a Mint Tx for 1000 units of our fungible asset
    let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();
    let tx_template =
        TransactionTemplate::MintFungibleAsset(fungible_asset, first_regular_account_id);

    println!("Minting Asset");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;

    // Check that note is committed
    println!("Fetching Pending Notes...");
    let notes = client.get_input_notes(NoteFilter::Pending).unwrap();
    assert!(notes.is_empty());

    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    let tx_template =
        TransactionTemplate::ConsumeNotes(first_regular_account_id, vec![notes[0].id()]);
    println!("Consuming Note...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request).await;

    let (regular_account, _seed) = client.get_account(first_regular_account_id).unwrap();

    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), MINT_AMOUNT);
    } else {
        panic!("ACCOUNT SHOULD HAVE A FUNGIBLE ASSET");
    }
}

async fn test_p2id_transfer() {
    let mut client = create_test_client();

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    // First Mint necesary token
    mint_note(&mut client, from_account_id, faucet_account_id).await;

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(PaymentTransactionData::new(
        Asset::Fungible(asset),
        from_account_id,
        to_account_id,
    ));
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

async fn test_p2idr_transfer() {
    let mut client = create_test_client();

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    // First Mint necesary token
    mint_note(&mut client, from_account_id, faucet_account_id).await;
    // Do a transfer from first account to second account with Recall. In this situation we'll do
    // the happy path where the `to_account_id` consumes the note
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
        Err(ClientError::TransactionExecutionError(
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
//   (ie, a word of 4 felts that represent [92,92,92,92])
//
// - Create another transaction that consumes this note with custom code. This custom code only
//   asserts that the {asserted_value} parameter is 0. To test this we first execute with
//   an incorrect value passed in, and after that we try again with the correct value.
//
// Because it's currently not possible to create/consume notes without assets, the P2ID code
// is used as the base for the note code.

async fn test_transaction_request() {
    let mut client = create_test_client();

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

    std::thread::sleep(Duration::from_secs(8));
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
    let note = create_custom_note(faucet_account_id, target_account_id, &mut random_coin);

    let recipient = note
        .recipient()
        .iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".");

    let code = "
    use.miden::contracts::faucets::basic_fungible->faucet
    use.miden::contracts::auth::basic->auth_tx
    
    begin
        push.{recipient}
        push.{tag}
        push.{amount}
        call.faucet::distribute
    
        call.auth_tx::auth_tx_rpo_falcon512
        dropw dropw
    end
    "
    .replace("{recipient}", &recipient)
    .replace("{tag}", &Felt::new(Into::<u64>::into(target_account_id)).to_string())
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
    faucet_account_id: AccountId,
    target_account_id: AccountId,
    rng: &mut RpoRandomCoin,
) -> Note {
    let assembler = TransactionKernel::assembler();

    let recipient = [Felt::new(9), Felt::new(12), Felt::new(18), Felt::new(3)]
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(".");

    let note_script =
        include_str!("asm/custom_p2id.masm").replace("{expected_note_arg}", &recipient);
    let note_script = ProgramAst::parse(&note_script).unwrap();
    let (note_script, _) = NoteScript::new(note_script, &assembler).unwrap();

    let inputs = [target_account_id.into()];
    let tag: Felt = target_account_id.into();
    let serial_num = rng.draw_word();

    Note::new(
        note_script,
        &inputs,
        &vec![FungibleAsset::new(faucet_account_id, 10).unwrap().into()],
        serial_num,
        faucet_account_id,
        tag,
    )
    .unwrap()
}
