// TESTS
// ================================================================================================
use crate::{
    client::{
        accounts::{AccountStorageMode, AccountTemplate},
        transactions::TransactionTemplate,
    },
    store::{
        accounts::AuthInfo,
        mock_executor_data_store::MockDataStore,
        notes::{InputNoteFilter, InputNoteRecord},
        tests::create_test_client,
    },
};

use assembly::ast::{AstSerdeOptions, ModuleAst};
use crypto::{dsa::rpo_falcon512::KeyPair, Felt, FieldElement, Word};
use miden_lib::transaction::{TransactionKernel};
use mock::{
    constants::{generate_account_seed, AccountSeedType},
    mock::{
        account::{self, mock_account, MockAccountType},
        notes::AssetPreservationStatus,
        transaction::{mock_inputs},
    },
};
use objects::{
    accounts::{AccountId, AccountStub},
    assets::{FungibleAsset, TokenSymbol},
    transaction::{InputNotes},
};

#[tokio::test]
async fn test_input_notes_round_trip() {
    // generate test client with a random store name
    let mut client = create_test_client();

    // generate test data
    let transaction_inputs = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );
    let _recorded_notes = transaction_inputs.input_notes();

    // insert notes into database
    for note in transaction_inputs.input_notes().iter().cloned() {
        client.import_input_note(note.into()).unwrap();
    }

    // retrieve notes from database
    let retrieved_notes = client.get_input_notes(InputNoteFilter::Committed).unwrap();

    let recorded_notes: Vec<InputNoteRecord> = transaction_inputs
        .input_notes()
        .iter()
        .map(|n| n.clone().into())
        .collect();
    // compare notes
    for (recorded_note, retrieved_note) in recorded_notes.iter().zip(retrieved_notes) {
        assert_eq!(recorded_note.note_id(), retrieved_note.note_id());
    }
}

#[tokio::test]
async fn test_get_input_note() {
    // generate test client with a random store name
    let mut client = create_test_client();

    // generate test data
    let transaction_inputs = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );
    let _recorded_notes: InputNotes = transaction_inputs.input_notes().clone();
    let recorded_notes: InputNotes = transaction_inputs.input_notes().clone();

    // insert note into database
    client
        .import_input_note(recorded_notes.get_note(0).clone().into())
        .unwrap();

    // retrieve note from database
    let retrieved_note = client
        .get_input_note(recorded_notes.get_note(0).note().id())
        .unwrap();

    let recorded_note: InputNoteRecord = recorded_notes.get_note(0).clone().into();
    assert_eq!(recorded_note.note_id(), retrieved_note.note_id())
}

#[tokio::test]
async fn insert_basic_account() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let account_template = AccountTemplate::BasicWallet {
        mutable_code: true,
        storage_mode: AccountStorageMode::Local,
    };

    // Insert Account
    let account_insert_result = client.new_account(account_template);
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account_by_id(account.id());
    assert!(fetched_account_data.is_ok());

    let (fetched_account, fetched_account_seed) = fetched_account_data.unwrap();
    // Validate stub has matching data
    assert_eq!(account.id(), fetched_account.id());
    assert_eq!(account.nonce(), fetched_account.nonce());
    assert_eq!(account.vault(), fetched_account.vault());
    assert_eq!(account.storage().root(), fetched_account.storage().root());
    assert_eq!(account.code().root(), fetched_account.code().root());

    // Validate seed matches
    assert_eq!(account_seed, fetched_account_seed);
}

#[tokio::test]
async fn insert_faucet_account() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let faucet_template = AccountTemplate::FungibleFaucet {
        token_symbol: TokenSymbol::new("TEST").unwrap(),
        decimals: 10,
        max_supply: 9999999999,
        storage_mode: AccountStorageMode::Local,
    };

    // Insert Account
    let account_insert_result = client.new_account(faucet_template);
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account_by_id(account.id());
    assert!(fetched_account_data.is_ok());

    let (fetched_account, fetched_account_seed) = fetched_account_data.unwrap();
    // Validate stub has matching data
    assert_eq!(account.id(), fetched_account.id());
    assert_eq!(account.nonce(), fetched_account.nonce());
    assert_eq!(account.vault(), fetched_account.vault());
    assert_eq!(account.storage(), fetched_account.storage());
    assert_eq!(account.code().root(), fetched_account.code().root());

    // Validate seed matches
    assert_eq!(account_seed, fetched_account_seed);
}

#[tokio::test]
async fn insert_same_account_twice_fails() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let assembler = TransactionKernel::assembler();

    let (account_id, account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);
    let account = account::mock_account(Some(account_id.into()), Felt::ZERO, None, &assembler);

    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();

    assert!(client
        .insert_account(&account, account_seed, &AuthInfo::RpoFalcon512(key_pair))
        .is_ok());
    assert!(client
        .insert_account(&account, account_seed, &AuthInfo::RpoFalcon512(key_pair))
        .is_err());
}

#[tokio::test]
async fn test_acc_code() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let assembler = TransactionKernel::assembler();
    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();

    let (account_id, account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);

    let account = account::mock_account(Some(account_id.into()), Felt::ZERO, None, &assembler);

    let mut account_module = account.code().module().clone();

    // this is needed due to the reconstruction not including source locations
    account_module.clear_locations();
    account_module.clear_imports();

    let account_module_bytes = account_module.to_bytes(AstSerdeOptions {
        serialize_imports: true,
    });
    let reconstructed_ast = ModuleAst::from_bytes(&account_module_bytes).unwrap();
    assert_eq!(account_module, reconstructed_ast);

    client
        .insert_account(&account, account_seed, &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();
    let (retrieved_acc, _) = client.get_account_by_id(account_id).unwrap();

    let mut account_module = account.code().module().clone();
    account_module.clear_locations();
    account_module.clear_imports();
    assert_eq!(
        *account_module.procs(),
        *retrieved_acc.code().module().procs()
    );
}

#[tokio::test]
async fn test_get_account_by_id() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let assembler = TransactionKernel::assembler();

    let (account_id, account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);
    let account = account::mock_account(Some(account_id.into()), Felt::ZERO, None, &assembler);

    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();

    client
        .insert_account(&account, account_seed, &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();

    // Retrieving an existing account should succeed
    let (acc_from_db, _account_seed) = match client.get_account_stub_by_id(account.id()) {
        Ok(account) => account,
        Err(err) => panic!("Error retrieving account: {}", err),
    };
    assert_eq!(AccountStub::from(account), acc_from_db);

    // Retrieving a non existing account should fail
    let hex = format!("0x{}", "1".repeat(16));
    let invalid_id = AccountId::from_hex(&hex).unwrap();
    assert!(client.get_account_stub_by_id(invalid_id).is_err());
}

#[tokio::test]
async fn test_sync_state() {
    // generate test client with a random store name
    let mut client = create_test_client();

    // generate test data
    crate::mock::insert_mock_data(&mut client).await;

    // assert that we have no consumed nor pending notes prior to syncing state
    assert_eq!(
        client
            .get_input_notes(InputNoteFilter::Consumed)
            .unwrap()
            .len(),
        0
    );

    let pending_notes = client.get_input_notes(InputNoteFilter::Pending).unwrap();

    // sync state
    let block_num: u32 = client.sync_state().await.unwrap();

    // verify that the client is synced to the latest block
    assert_eq!(
        block_num,
        client
            .rpc_api
            .sync_state_requests
            .first_key_value()
            .unwrap()
            .1
            .chain_tip
    );

    // verify that we now have one consumed note after syncing state
    assert_eq!(
        client
            .get_input_notes(InputNoteFilter::Consumed)
            .unwrap()
            .len(),
        1
    );

    // verify that the pending note we had is now committed
    assert_ne!(
        client.get_input_notes(InputNoteFilter::Committed).unwrap(),
        pending_notes
    );

    // verify that the latest block number has been updated
    assert_eq!(
        client.get_sync_height().unwrap(),
        client
            .rpc_api
            .sync_state_requests
            .first_key_value()
            .unwrap()
            .1
            .chain_tip
    );
}

#[tokio::test]
async fn test_add_tag() {
    // generate test client with a random store name
    let mut client = create_test_client();

    // assert that no tags are being tracked
    assert_eq!(client.get_note_tags().unwrap().len(), 0);

    // add a tag
    const TAG_VALUE_1: u64 = 1;
    const TAG_VALUE_2: u64 = 2;
    client.add_note_tag(TAG_VALUE_1).unwrap();
    client.add_note_tag(TAG_VALUE_2).unwrap();

    // verify that the tag is being tracked
    assert_eq!(
        client.get_note_tags().unwrap(),
        vec![TAG_VALUE_1, TAG_VALUE_2]
    );

    // attempt to add the same tag again
    client.add_note_tag(TAG_VALUE_1).unwrap();

    // verify that the tag is still being tracked only once
    assert_eq!(
        client.get_note_tags().unwrap(),
        vec![TAG_VALUE_1, TAG_VALUE_2]
    );
}

#[tokio::test]
#[ignore = "currently fails with PhantomCallsNotAllowed"]
async fn test_mint_transaction() {
    const FAUCET_ID: u64 = 10347894387879516201u64;
    const FAUCET_SEED: Word = [Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::ZERO];

    // generate test client with a random store name
    let mut client = create_test_client();

    let (faucet, _seed) = client
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("TST").unwrap(),
            decimals: 10u8,
            max_supply: 1000u64,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    let faucet = mock_account(
        Some(FAUCET_ID),
        Felt::new(10u64),
        Some(faucet.code().clone()),
        &TransactionKernel::assembler(),
    );

    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();
    client
        .store
        .insert_account(&faucet, FAUCET_SEED, &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();
    client.set_data_store(MockDataStore::with_existing(faucet.clone(), None, None));

    // Test submitting a mint transaction

    let transaction_template = TransactionTemplate::MintFungibleAsset {
        asset: FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        target_account_id: AccountId::from_hex("0x168187d729b31a84").unwrap(),
    };

    let transaction = client.new_transaction(transaction_template).unwrap();
    assert!(transaction
        .executed_transaction()
        .account_delta()
        .nonce()
        .is_some());
}

#[tokio::test]
#[ignore = "currently fails because executor's DuplicateProcName error, see https://github.com/0xPolygonMiden/miden-base/issues/443"]
async fn test_consume_all_transaction() {
    // generate test client with a random store name
    let mut client = create_test_client();

    // generate test data
    let transaction_inputs = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    let recorded_notes: InputNotes = transaction_inputs.input_notes().clone();
    let mut notes_for_data_store = vec![];
    for note in recorded_notes {
        notes_for_data_store.push(note.note().clone());
    }

    let (account, seed) = client
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    let data_store =
        MockDataStore::with_existing(account.clone(), Some(seed), Some(notes_for_data_store));
    client.set_data_store(data_store);

    let transaction_template = TransactionTemplate::ConsumeAllNotes(account.id());
    client.new_transaction(transaction_template).unwrap();
}
