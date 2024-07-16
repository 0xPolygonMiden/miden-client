use alloc::vec::Vec;

// TESTS
// ================================================================================================
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, AccountId, AccountStorageType,
        AccountStub, AuthSecretKey,
    },
    assembly::{AstSerdeOptions, ModuleAst},
    assets::{FungibleAsset, TokenSymbol},
    crypto::dsa::rpo_falcon512::SecretKey,
    notes::{NoteFile, NoteTag},
    Word,
};

use crate::{
    accounts::AccountTemplate,
    mock::{
        create_test_client, get_account_with_default_account_code, mock_full_chain_mmr_and_notes,
        mock_fungible_faucet_account, mock_notes, ACCOUNT_ID_REGULAR,
    },
    store::{InputNoteRecord, NoteFilter},
    transactions::transaction_request::TransactionTemplate,
};

#[tokio::test]
async fn test_input_notes_round_trip() {
    // generate test client with a random store name
    let mut client = create_test_client();

    // generate test data

    let assembler = TransactionKernel::assembler();
    let (consumed_notes, _created_notes) = mock_notes(&assembler);
    let (_, consumed_notes, ..) = mock_full_chain_mmr_and_notes(consumed_notes);

    // insert notes into database
    for note in consumed_notes.iter() {
        client
            .import_note(NoteFile::NoteWithProof(
                note.note().clone(),
                note.proof().expect("These notes should be authenticated").clone(),
            ))
            .await
            .unwrap();
    }

    // retrieve notes from database
    let retrieved_notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert_eq!(retrieved_notes.len(), consumed_notes.len());

    let recorded_notes: Vec<InputNoteRecord> =
        consumed_notes.iter().map(|n| n.clone().into()).collect();
    // compare notes
    for (recorded_note, retrieved_note) in recorded_notes.iter().zip(retrieved_notes) {
        assert_eq!(recorded_note.id(), retrieved_note.id());
    }
}

#[tokio::test]
async fn test_get_input_note() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let assembler = TransactionKernel::assembler();
    let (_consumed_notes, created_notes) = mock_notes(&assembler);

    // insert Note into database
    let note: InputNoteRecord = created_notes.first().unwrap().clone().into();
    client.import_note(NoteFile::NoteDetails(note.into(), None)).await.unwrap();

    // retrieve note from database
    let retrieved_note =
        client.get_input_note(created_notes.first().unwrap().clone().id()).unwrap();

    let recorded_note: InputNoteRecord = created_notes.first().unwrap().clone().into();
    assert_eq!(recorded_note.id(), retrieved_note.id());
}

#[tokio::test]
async fn insert_basic_account() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let account_template = AccountTemplate::BasicWallet {
        mutable_code: true,
        storage_type: AccountStorageType::OffChain,
    };

    // Insert Account
    let account_insert_result = client.new_account(account_template);
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account(account.id());
    assert!(fetched_account_data.is_ok());

    let (fetched_account, fetched_account_seed) = fetched_account_data.unwrap();
    // Validate stub has matching data
    assert_eq!(account.id(), fetched_account.id());
    assert_eq!(account.nonce(), fetched_account.nonce());
    assert_eq!(account.vault(), fetched_account.vault());
    assert_eq!(account.storage().root(), fetched_account.storage().root());
    assert_eq!(account.code().root(), fetched_account.code().root());

    // Validate seed matches
    assert_eq!(account_seed, fetched_account_seed.unwrap());
}

#[tokio::test]
async fn insert_faucet_account() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let faucet_template = AccountTemplate::FungibleFaucet {
        token_symbol: TokenSymbol::new("TEST").unwrap(),
        decimals: 10,
        max_supply: 9999999999,
        storage_type: AccountStorageType::OffChain,
    };

    // Insert Account
    let account_insert_result = client.new_account(faucet_template);
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account(account.id());
    assert!(fetched_account_data.is_ok());

    let (fetched_account, fetched_account_seed) = fetched_account_data.unwrap();
    // Validate stub has matching data
    assert_eq!(account.id(), fetched_account.id());
    assert_eq!(account.nonce(), fetched_account.nonce());
    assert_eq!(account.vault(), fetched_account.vault());
    assert_eq!(account.storage(), fetched_account.storage());
    assert_eq!(account.code().root(), fetched_account.code().root());

    // Validate seed matches
    assert_eq!(account_seed, fetched_account_seed.unwrap());
}

#[tokio::test]
async fn insert_same_account_twice_fails() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let account = get_account_with_default_account_code(
        AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap(),
        Word::default(),
        None,
    );

    let key_pair = SecretKey::new();

    assert!(client
        .insert_account(
            &account,
            Some(Word::default()),
            &AuthSecretKey::RpoFalcon512(key_pair.clone())
        )
        .is_ok());
    assert!(client
        .insert_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair))
        .is_err());
}

#[tokio::test]
async fn test_account_code() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let key_pair = SecretKey::new();

    let account = get_account_with_default_account_code(
        AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap(),
        Word::default(),
        None,
    );

    let mut account_module = account.code().module().clone();

    // this is needed due to the reconstruction not including source locations
    account_module.clear_locations();
    account_module.clear_imports();

    let account_module_bytes = account_module.to_bytes(AstSerdeOptions { serialize_imports: true });
    let reconstructed_ast = ModuleAst::from_bytes(&account_module_bytes).unwrap();
    assert_eq!(account_module, reconstructed_ast);

    client
        .insert_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();
    let (retrieved_acc, _) = client.get_account(account.id()).unwrap();

    let mut account_module = account.code().module().clone();
    account_module.clear_locations();
    account_module.clear_imports();
    assert_eq!(*account_module.procs(), *retrieved_acc.code().module().procs());
}

#[tokio::test]
async fn test_get_account_by_id() {
    // generate test client with a random store name
    let mut client = create_test_client();

    let account = get_account_with_default_account_code(
        AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap(),
        Word::default(),
        None,
    );

    let key_pair = SecretKey::new();

    client
        .insert_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair))
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

    // assert that we have no consumed nor expected notes prior to syncing state
    assert_eq!(client.get_input_notes(NoteFilter::Consumed).unwrap().len(), 0);

    let expected_notes = client.get_input_notes(NoteFilter::Expected).unwrap();

    // sync state
    let sync_details = client.sync_state().await.unwrap();

    // verify that the client is synced to the latest block
    assert_eq!(
        sync_details.block_num,
        client.rpc_api().state_sync_requests.first_key_value().unwrap().1.chain_tip
    );

    // verify that we now have one consumed note after syncing state
    assert_eq!(client.get_input_notes(NoteFilter::Consumed).unwrap().len(), 1);
    assert_eq!(sync_details.new_nullifiers, 1);

    // verify that the expected note we had is now committed
    assert_ne!(client.get_input_notes(NoteFilter::Committed).unwrap(), expected_notes);

    // verify that the latest block number has been updated
    assert_eq!(
        client.get_sync_height().unwrap(),
        client.rpc_api().state_sync_requests.first_key_value().unwrap().1.chain_tip
    );
}

#[tokio::test]
async fn test_sync_state_mmr() {
    // generate test client with a random store name
    let mut client = create_test_client();

    // generate test data
    let tracked_block_headers = crate::mock::insert_mock_data(&mut client).await;

    // sync state
    let sync_details = client.sync_state().await.unwrap();

    // verify that the client is synced to the latest block
    assert_eq!(
        sync_details.block_num,
        client.rpc_api().state_sync_requests.first_key_value().unwrap().1.chain_tip
    );

    // verify that the latest block number has been updated
    assert_eq!(
        client.get_sync_height().unwrap(),
        client.rpc_api().state_sync_requests.first_key_value().unwrap().1.chain_tip
    );

    // verify that we inserted the latest block into the db via the client
    let latest_block = client.get_sync_height().unwrap();
    assert_eq!(sync_details.block_num, latest_block);
    assert_eq!(
        tracked_block_headers[tracked_block_headers.len() - 1],
        client.get_block_headers(&[latest_block]).unwrap()[0].0
    );

    // Try reconstructing the chain_mmr from what's in the database
    let partial_mmr = client.build_current_partial_mmr(true).unwrap();

    // Since Mocked data contains three sync updates we should be "tracking" those blocks
    // However, remember that we don't actually update the partial_mmr with the latest block but up
    // to one block before instead. This is because the prologue will already build the
    // authentication path for that block.
    assert_eq!(partial_mmr.forest(), 7);
    assert!(partial_mmr.open(0).unwrap().is_none());
    assert!(partial_mmr.open(1).unwrap().is_none());
    assert!(partial_mmr.open(2).unwrap().is_some());
    assert!(partial_mmr.open(3).unwrap().is_none());
    assert!(partial_mmr.open(4).unwrap().is_some());
    assert!(partial_mmr.open(5).unwrap().is_none());
    assert!(partial_mmr.open(6).unwrap().is_some());

    // Ensure the proofs are valid
    let mmr_proof = partial_mmr.open(2).unwrap().unwrap();
    assert!(partial_mmr.peaks().verify(tracked_block_headers[0].hash(), mmr_proof));

    let mmr_proof = partial_mmr.open(4).unwrap().unwrap();
    assert!(partial_mmr.peaks().verify(tracked_block_headers[1].hash(), mmr_proof));
}

#[tokio::test]
async fn test_tags() {
    // generate test client with a random store name
    let mut client = create_test_client();

    // Assert that the store gets created with the tag 0 (used for notes consumable by any account)
    assert_eq!(client.get_note_tags().unwrap(), vec![]);

    // add a tag
    let tag_1: NoteTag = 1.into();
    let tag_2: NoteTag = 2.into();
    client.add_note_tag(tag_1).unwrap();
    client.add_note_tag(tag_2).unwrap();

    // verify that the tag is being tracked
    assert_eq!(client.get_note_tags().unwrap(), vec![tag_1, tag_2]);

    // attempt to add the same tag again
    client.add_note_tag(tag_1).unwrap();

    // verify that the tag is still being tracked only once
    assert_eq!(client.get_note_tags().unwrap(), vec![tag_1, tag_2]);

    // Try removing non-existent tag
    let tag_4: NoteTag = 4.into();
    client.remove_note_tag(tag_4).unwrap();

    // verify that the tracked tags are unchanged
    assert_eq!(client.get_note_tags().unwrap(), vec![tag_1, tag_2]);

    // remove second tag
    client.remove_note_tag(tag_1).unwrap();

    // verify that tag_1 is not tracked anymore
    assert_eq!(client.get_note_tags().unwrap(), vec![tag_2]);
}

#[tokio::test]
async fn test_mint_transaction() {
    const FAUCET_ID: u64 = ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN;
    const INITIAL_BALANCE: u64 = 1000;

    // generate test client with a random store name
    let mut client = create_test_client();

    // Faucet account generation
    let key_pair = SecretKey::new();

    let faucet = mock_fungible_faucet_account(
        AccountId::try_from(FAUCET_ID).unwrap(),
        INITIAL_BALANCE,
        key_pair.clone(),
    );

    client
        .store()
        .insert_account(&faucet, None, &AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();

    client.sync_state().await.unwrap();

    // Test submitting a mint transaction
    let transaction_template = TransactionTemplate::MintFungibleAsset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::from_hex("0x168187d729b31a84").unwrap(),
        miden_objects::notes::NoteType::Private,
    );

    let transaction_request = client.build_transaction_request(transaction_template).unwrap();

    let transaction = client.new_transaction(transaction_request, None).unwrap();
    assert!(transaction.executed_transaction().account_delta().nonce().is_some());
}

#[tokio::test]
async fn test_get_output_notes() {
    const FAUCET_ID: u64 = ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN;
    const INITIAL_BALANCE: u64 = 1000;

    // generate test client with a random store name
    let mut client = create_test_client();

    // Faucet account generation
    let key_pair = SecretKey::new();

    let faucet = mock_fungible_faucet_account(
        AccountId::try_from(FAUCET_ID).unwrap(),
        INITIAL_BALANCE,
        key_pair.clone(),
    );

    client
        .store()
        .insert_account(&faucet, None, &AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();

    client.sync_state().await.unwrap();

    // Test submitting a mint transaction
    let transaction_template = TransactionTemplate::MintFungibleAsset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::from_hex("0x168187d729b31a84").unwrap(),
        miden_objects::notes::NoteType::Private,
    );

    let transaction_request = client.build_transaction_request(transaction_template).unwrap();

    //Before executing transaction, there are no output notes
    assert!(client.get_output_notes(NoteFilter::All).unwrap().is_empty());

    let transaction = client.new_transaction(transaction_request, None).unwrap();
    let proven_transaction =
        client.prove_transaction(transaction.executed_transaction().clone()).unwrap();
    client.submit_transaction(transaction, proven_transaction).await.unwrap();

    // Check that there was an output note but it wasn't consumed
    assert!(client.get_output_notes(NoteFilter::Consumed).unwrap().is_empty());
    assert!(!client.get_output_notes(NoteFilter::All).unwrap().is_empty());
}

#[tokio::test]
async fn test_import_note_validation() {
    // generate test client
    let mut client = create_test_client();

    // generate test data
    let assembler = TransactionKernel::assembler();
    let (consumed_notes, created_notes) = mock_notes(&assembler);
    let (_, committed_notes, ..) = mock_full_chain_mmr_and_notes(consumed_notes);

    let committed_note: InputNoteRecord = committed_notes.first().unwrap().clone().into();
    let expected_note = InputNoteRecord::from(created_notes.first().unwrap().clone());

    client
        .import_note(NoteFile::NoteDetails(committed_note.clone().into(), None))
        .await
        .unwrap();
    assert!(client.import_note(NoteFile::NoteId(expected_note.id())).await.is_err());
    client
        .import_note(NoteFile::NoteDetails(expected_note.clone().into(), None))
        .await
        .unwrap();
    assert!(expected_note.inclusion_proof().is_none());
    assert!(committed_note.inclusion_proof().is_some());
}
