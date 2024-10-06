use alloc::vec::Vec;

// TESTS
// ================================================================================================
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
        },
        Account, AccountCode, AccountHeader, AccountId, AccountStorageMode, AuthSecretKey,
    },
    assets::{FungibleAsset, TokenSymbol},
    crypto::dsa::rpo_falcon512::SecretKey,
    notes::{NoteFile, NoteTag},
    Felt, FieldElement, Word,
};
use miden_tx::utils::{Deserializable, Serializable};

use crate::{
    accounts::AccountTemplate,
    mock::create_test_client,
    rpc::NodeRpcClient,
    store::{InputNoteRecord, NoteFilter},
    transactions::TransactionRequest,
};

#[tokio::test]
async fn test_input_notes_round_trip() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client();

    client
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: true,
            storage_mode: AccountStorageMode::Public,
        })
        .unwrap();
    // generate test data
    let available_notes = [rpc_api.get_note_at(0), rpc_api.get_note_at(1)];

    // insert notes into database
    for note in available_notes.iter() {
        client
            .import_note(NoteFile::NoteWithProof(
                note.note().clone(),
                note.proof().expect("These notes should be authenticated").clone(),
            ))
            .await
            .unwrap();
    }

    // retrieve notes from database
    // TODO: Once we get more specific filters this query should only get unverified notes.
    let retrieved_notes = client.get_input_notes(NoteFilter::All).unwrap();
    assert_eq!(retrieved_notes.len(), 2);

    let recorded_notes: Vec<InputNoteRecord> =
        available_notes.iter().map(|n| n.clone().into()).collect();
    // compare notes
    for (recorded_note, retrieved_note) in recorded_notes.iter().zip(retrieved_notes) {
        assert_eq!(recorded_note.id(), retrieved_note.id());
    }
}

#[tokio::test]
async fn test_get_input_note() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client();
    // Get note from mocked RPC backend since any note works here
    let original_note = rpc_api.get_note_at(0).note().clone();

    // insert Note into database
    let note: InputNoteRecord = original_note.clone().into();
    client
        .import_note(NoteFile::NoteDetails {
            details: note.into(),
            tag: None,
            after_block_num: 0,
        })
        .await
        .unwrap();

    // retrieve note from database
    let retrieved_note = client.get_input_note(original_note.id()).unwrap();

    let recorded_note: InputNoteRecord = original_note.into();
    assert_eq!(recorded_note.id(), retrieved_note.id());
}

#[tokio::test]
async fn insert_basic_account() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client();

    let account_template = AccountTemplate::BasicWallet {
        mutable_code: true,
        storage_mode: AccountStorageMode::Private,
    };

    // Insert Account
    let account_insert_result = client.new_account(account_template);
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account(account.id());
    assert!(fetched_account_data.is_ok());

    let (fetched_account, fetched_account_seed) = fetched_account_data.unwrap();
    // Validate header has matching data
    assert_eq!(account.id(), fetched_account.id());
    assert_eq!(account.nonce(), fetched_account.nonce());
    assert_eq!(account.vault(), fetched_account.vault());
    assert_eq!(account.storage().commitment(), fetched_account.storage().commitment());
    assert_eq!(account.code().commitment(), fetched_account.code().commitment());

    // Validate seed matches
    assert_eq!(account_seed, fetched_account_seed.unwrap());
}

#[tokio::test]
async fn insert_faucet_account() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client();

    let faucet_template = AccountTemplate::FungibleFaucet {
        token_symbol: TokenSymbol::new("TEST").unwrap(),
        decimals: 10,
        max_supply: 9999999999,
        storage_mode: AccountStorageMode::Private,
    };

    // Insert Account
    let account_insert_result = client.new_account(faucet_template);
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account(account.id());
    assert!(fetched_account_data.is_ok());

    let (fetched_account, fetched_account_seed) = fetched_account_data.unwrap();
    // Validate header has matching data
    assert_eq!(account.id(), fetched_account.id());
    assert_eq!(account.nonce(), fetched_account.nonce());
    assert_eq!(account.vault(), fetched_account.vault());
    assert_eq!(account.storage(), fetched_account.storage());
    assert_eq!(account.code().commitment(), fetched_account.code().commitment());

    // Validate seed matches
    assert_eq!(account_seed, fetched_account_seed.unwrap());
}

#[tokio::test]
async fn insert_same_account_twice_fails() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client();

    let account = Account::mock(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
        Felt::new(2),
        TransactionKernel::testing_assembler(),
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
    let (mut client, rpc_api) = create_test_client();

    let key_pair = SecretKey::new();

    let account = Account::mock(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        Felt::ZERO,
        TransactionKernel::testing_assembler(),
    );

    let account_code = account.code();

    let account_code_bytes = account_code.to_bytes();

    let reconstructed_code = AccountCode::read_from_bytes(&account_code_bytes).unwrap();
    assert_eq!(*account_code, reconstructed_code);

    client
        .insert_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();
    let (retrieved_acc, _) = client.get_account(account.id()).unwrap();
    assert_eq!(*account.code(), *retrieved_acc.code());
}

#[tokio::test]
async fn test_get_account_by_id() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client();

    let account = Account::mock(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
        Felt::new(10),
        TransactionKernel::assembler(),
    );

    let key_pair = SecretKey::new();

    client
        .insert_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();

    // Retrieving an existing account should succeed
    let (acc_from_db, _account_seed) = match client.get_account_header_by_id(account.id()) {
        Ok(account) => account,
        Err(err) => panic!("Error retrieving account: {}", err),
    };
    assert_eq!(AccountHeader::from(account), acc_from_db);

    // Retrieving a non existing account should fail
    let hex = format!("0x{}", "1".repeat(16));
    let invalid_id = AccountId::from_hex(&hex).unwrap();
    assert!(client.get_account_header_by_id(invalid_id).is_err());
}

#[tokio::test]
async fn test_sync_state() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client();

    // Import first mockchain note as expected
    let expected_note = client.rpc_api.get_note_at(1).note().clone();
    client.store.upsert_input_note(expected_note.clone().into()).unwrap();

    // assert that we have no consumed nor expected notes prior to syncing state
    assert_eq!(client.get_input_notes(NoteFilter::Consumed).unwrap().len(), 0);
    assert_eq!(client.get_input_notes(NoteFilter::Expected).unwrap().len(), 1);
    assert_eq!(client.get_input_notes(NoteFilter::Committed).unwrap().len(), 0);

    let expected_notes = client.get_input_notes(NoteFilter::Expected).unwrap();

    // sync state
    let sync_details = client.sync_state().await.unwrap();

    // verify that the client is synced to the latest block
    assert_eq!(sync_details.block_num, rpc_api.blocks.last().unwrap().header().block_num());

    // verify that the expected note we had is now committed
    assert_ne!(client.get_input_notes(NoteFilter::Committed).unwrap(), expected_notes);

    // verify that we now have one consumed note after syncing state
    assert_eq!(client.get_input_notes(NoteFilter::Consumed).unwrap().len(), 1);
    assert_eq!(sync_details.consumed_notes.len(), 1);

    // verify that the latest block number has been updated
    assert_eq!(
        client.get_sync_height().unwrap(),
        rpc_api.blocks.last().unwrap().header().block_num()
    );
}

#[tokio::test]
async fn test_sync_state_mmr() {
    // generate test client with a random store name
    let (mut client, mut rpc_api) = create_test_client();
    // Import note and create wallet so that synced notes do not get discarded (due to being
    // irrelevant)
    client
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Private,
        })
        .unwrap();
    for (_, n) in client.rpc_api.notes.iter() {
        client.store.upsert_input_note(n.note().clone().into()).unwrap();
    }

    // sync state
    let sync_details = client.sync_state().await.unwrap();

    // verify that the client is synced to the latest block
    assert_eq!(sync_details.block_num, rpc_api.blocks.last().unwrap().header().block_num());

    // verify that the latest block number has been updated
    assert_eq!(
        client.get_sync_height().unwrap(),
        rpc_api.blocks.last().unwrap().header().block_num()
    );

    // verify that we inserted the latest block into the DB via the client
    let latest_block = client.get_sync_height().unwrap();
    assert_eq!(sync_details.block_num, latest_block);
    assert_eq!(
        rpc_api.blocks.last().unwrap().hash(),
        client.get_block_headers(&[latest_block]).unwrap()[0].0.hash()
    );

    // Try reconstructing the chain_mmr from what's in the database
    let partial_mmr = client.build_current_partial_mmr(true).unwrap();
    assert_eq!(partial_mmr.forest(), 6);
    assert!(partial_mmr.open(0).unwrap().is_none());
    assert!(partial_mmr.open(1).unwrap().is_some());
    assert!(partial_mmr.open(2).unwrap().is_none());
    assert!(partial_mmr.open(3).unwrap().is_none());
    assert!(partial_mmr.open(4).unwrap().is_some());
    assert!(partial_mmr.open(5).unwrap().is_none());

    // Ensure the proofs are valid
    let mmr_proof = partial_mmr.open(1).unwrap().unwrap();
    let (block_1, _) = rpc_api.get_block_header_by_number(Some(1), false).await.unwrap();
    assert!(partial_mmr.peaks().verify(block_1.hash(), mmr_proof));

    let mmr_proof = partial_mmr.open(4).unwrap().unwrap();
    let (block_4, _) = rpc_api.get_block_header_by_number(Some(4), false).await.unwrap();
    assert!(partial_mmr.peaks().verify(block_4.hash(), mmr_proof));
}

#[tokio::test]
async fn test_tags() {
    // generate test client with a random store name
    let (mut client, _rpc_api) = create_test_client();

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
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client();

    // Faucet account generation
    let (faucet, _seed) = client
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: "TST".try_into().unwrap(),
            decimals: 3,
            max_supply: 10000,
            storage_mode: AccountStorageMode::Private,
        })
        .unwrap();

    client.sync_state().await.unwrap();

    // Test submitting a mint transaction
    let transaction_request = TransactionRequest::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::from_hex("0x168187d729b31a84").unwrap(),
        miden_objects::notes::NoteType::Private,
        client.rng(),
    )
    .unwrap();

    let transaction = client.new_transaction(faucet.id(), transaction_request).unwrap();

    assert!(transaction.executed_transaction().account_delta().nonce().is_some());
}

#[tokio::test]
async fn test_get_output_notes() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client();
    client.sync_state().await.unwrap();

    // Faucet account generation
    let (faucet, _seed) = client
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: "TST".try_into().unwrap(),
            decimals: 3,
            max_supply: 10000,
            storage_mode: AccountStorageMode::Private,
        })
        .unwrap();

    // Test submitting a mint transaction
    let transaction_request = TransactionRequest::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::from_hex("0x0123456789abcdef").unwrap(),
        miden_objects::notes::NoteType::Private,
        client.rng(),
    )
    .unwrap();

    //Before executing transaction, there are no output notes
    assert!(client.get_output_notes(NoteFilter::All).unwrap().is_empty());

    let transaction = client.new_transaction(faucet.id(), transaction_request).unwrap();
    client.submit_transaction(transaction).await.unwrap();

    // Check that there was an output note but it wasn't consumed
    assert!(client.get_output_notes(NoteFilter::Consumed).unwrap().is_empty());
    assert!(!client.get_output_notes(NoteFilter::All).unwrap().is_empty());
}

#[tokio::test]
async fn test_import_note_validation() {
    // generate test client
    let (mut client, rpc_api) = create_test_client();

    // generate test data
    let committed_note: InputNoteRecord = rpc_api.get_note_at(0).into();
    let expected_note: InputNoteRecord = rpc_api.get_note_at(1).note().clone().into();

    client
        .import_note(NoteFile::NoteDetails {
            details: committed_note.clone().into(),
            after_block_num: 0,
            tag: None,
        })
        .await
        .unwrap();
    assert!(client.import_note(NoteFile::NoteId(expected_note.id())).await.is_err());
    client
        .import_note(NoteFile::NoteDetails {
            details: expected_note.clone().into(),
            after_block_num: 0,
            tag: None,
        })
        .await
        .unwrap();

    assert!(expected_note.inclusion_proof().is_none());
    assert!(committed_note.inclusion_proof().is_some());
}
