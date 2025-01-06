use alloc::vec::Vec;

// TESTS
// ================================================================================================
use miden_lib::{
    accounts::{auth::RpoFalcon512, faucets::BasicFungibleFaucet, wallets::BasicWallet},
    notes::utils,
    transaction::TransactionKernel,
};
use miden_objects::{
    accounts::{
        Account, AccountBuilder, AccountCode, AccountHeader, AccountId, AccountStorageMode,
        AccountType, AuthSecretKey,
    },
    assets::{FungibleAsset, TokenSymbol},
    crypto::{dsa::rpo_falcon512::SecretKey, rand::FeltRng},
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteFile, NoteMetadata, NoteTag,
        NoteType,
    },
    testing::account_id::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
    },
    transaction::OutputNote,
    Felt, FieldElement, Word, ZERO,
};
use miden_tx::utils::{Deserializable, Serializable};

use crate::{
    mock::create_test_client,
    rpc::NodeRpcClient,
    store::{InputNoteRecord, NoteFilter, Store, StoreError},
    transactions::{
        TransactionRequestBuilder, TransactionRequestError, TransactionScriptBuilderError,
    },
    Client, ClientError,
};

async fn insert_new_wallet<R: FeltRng>(
    client: &mut Client<R>,
    storage_mode: AccountStorageMode,
) -> Result<(Account, Word), ClientError> {
    let key_pair = SecretKey::with_rng(&mut client.rng);

    let mut init_seed = [0u8; 32];
    client.rng.fill_bytes(&mut init_seed);

    let anchor_block = client.get_latest_epoch_block().await.unwrap();

    let (account, seed) = AccountBuilder::new()
        .init_seed(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(storage_mode)
        .with_component(RpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    client
        .add_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair.clone()), false)
        .await?;

    Ok((account, seed))
}

async fn insert_new_fungible_faucet<R: FeltRng>(
    client: &mut Client<R>,
    storage_mode: AccountStorageMode,
) -> Result<(Account, Word), ClientError> {
    let key_pair = SecretKey::with_rng(&mut client.rng);

    // we need to use an initial seed to create the wallet account
    let mut init_seed = [0u8; 32];
    client.rng.fill_bytes(&mut init_seed);

    let symbol = TokenSymbol::new("TEST").unwrap();
    let max_supply = Felt::try_from(9999999_u64.to_le_bytes().as_slice())
        .expect("u64 can be safely converted to a field element");

    let anchor_block = client.get_latest_epoch_block().await.unwrap();

    let (account, seed) = AccountBuilder::new()
        .init_seed(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(AccountType::FungibleFaucet)
        .storage_mode(storage_mode)
        .with_component(RpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicFungibleFaucet::new(symbol, 10, max_supply).unwrap())
        .build()
        .unwrap();

    client
        .add_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair), false)
        .await?;
    Ok((account, seed))
}

#[tokio::test]
async fn test_input_notes_round_trip() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client().await;

    insert_new_wallet(&mut client, AccountStorageMode::Private).await.unwrap();
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
    let retrieved_notes = client.get_input_notes(NoteFilter::All).await.unwrap();
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
    let (mut client, rpc_api) = create_test_client().await;
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
    let retrieved_note = client.get_input_note(original_note.id()).await.unwrap();

    let recorded_note: InputNoteRecord = original_note.into();
    assert_eq!(recorded_note.id(), retrieved_note.id());
}

#[tokio::test]
async fn insert_basic_account() {
    // generate test client with a random store name
    let (mut client, _rpc_api) = create_test_client().await;

    // Insert Account
    let account_insert_result = insert_new_wallet(&mut client, AccountStorageMode::Private).await;
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account(account.id()).await;
    assert!(fetched_account_data.is_ok());

    let fetched_account = fetched_account_data.unwrap();
    let fetched_account_seed = fetched_account.seed().cloned();
    let fetched_account: Account = fetched_account.into();

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
    let (mut client, _rpc_api) = create_test_client().await;

    // Insert Account
    let account_insert_result =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private).await;
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account(account.id()).await;
    assert!(fetched_account_data.is_ok());

    let fetched_account = fetched_account_data.unwrap();
    let fetched_account_seed = fetched_account.seed().cloned();
    let fetched_account: Account = fetched_account.into();

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
    let (mut client, _rpc_api) = create_test_client().await;

    let account = Account::mock(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
        Felt::new(2),
        TransactionKernel::testing_assembler(),
    );

    let key_pair = SecretKey::new();

    assert!(client
        .add_account(
            &account,
            Some(Word::default()),
            &AuthSecretKey::RpoFalcon512(key_pair.clone()),
            false
        )
        .await
        .is_ok());
    assert!(client
        .add_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair), false)
        .await
        .is_err());
}

#[tokio::test]
async fn test_account_code() {
    // generate test client with a random store name
    let (mut client, _rpc_api) = create_test_client().await;

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
        .add_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair), false)
        .await
        .unwrap();
    let retrieved_acc = client.get_account(account.id()).await.unwrap();
    assert_eq!(*account.code(), *retrieved_acc.account().code());
}

#[tokio::test]
async fn test_get_account_by_id() {
    // generate test client with a random store name
    let (mut client, _rpc_api) = create_test_client().await;

    let account = Account::mock(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
        Felt::new(10),
        TransactionKernel::assembler(),
    );

    let key_pair = SecretKey::new();

    client
        .add_account(&account, Some(Word::default()), &AuthSecretKey::RpoFalcon512(key_pair), false)
        .await
        .unwrap();

    // Retrieving an existing account should succeed
    let (acc_from_db, _account_seed) = match client.get_account_header_by_id(account.id()).await {
        Ok(account) => account,
        Err(err) => panic!("Error retrieving account: {}", err),
    };
    assert_eq!(AccountHeader::from(account), acc_from_db);

    // Retrieving a non existing account should fail
    let invalid_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
    assert!(client.get_account_header_by_id(invalid_id).await.is_err());
}

#[tokio::test]
async fn test_sync_state() {
    // generate test client with a random store name
    let (mut client, rpc_api) = create_test_client().await;

    // Import first mockchain note as expected
    let expected_notes =
        rpc_api.notes.values().map(|n| n.note().clone().into()).collect::<Vec<_>>();
    Store::upsert_input_notes(client.store.as_ref(), &expected_notes).await.unwrap();

    // assert that we have no consumed nor expected notes prior to syncing state
    assert_eq!(client.get_input_notes(NoteFilter::Consumed).await.unwrap().len(), 0);
    assert_eq!(
        client.get_input_notes(NoteFilter::Expected).await.unwrap().len(),
        expected_notes.len()
    );
    assert_eq!(client.get_input_notes(NoteFilter::Committed).await.unwrap().len(), 0);

    // sync state
    let sync_details = client.sync_state().await.unwrap();

    // verify that the client is synced to the latest block
    assert_eq!(sync_details.block_num, rpc_api.blocks.last().unwrap().header().block_num());

    // verify that we now have one committed note after syncing state
    assert_eq!(client.get_input_notes(NoteFilter::Committed).await.unwrap().len(), 1);

    // verify that we now have one consumed note after syncing state
    assert_eq!(client.get_input_notes(NoteFilter::Consumed).await.unwrap().len(), 1);
    assert_eq!(sync_details.consumed_notes.len(), 1);

    // verify that the latest block number has been updated
    assert_eq!(
        client.get_sync_height().await.unwrap(),
        rpc_api.blocks.last().unwrap().header().block_num()
    );
}

#[tokio::test]
async fn test_sync_state_mmr() {
    // generate test client with a random store name
    let (mut client, mut rpc_api) = create_test_client().await;
    // Import note and create wallet so that synced notes do not get discarded (due to being
    // irrelevant)
    insert_new_wallet(&mut client, AccountStorageMode::Private).await.unwrap();

    let notes = rpc_api.notes.values().map(|n| n.note().clone().into()).collect::<Vec<_>>();
    Store::upsert_input_notes(client.store.as_ref(), &notes).await.unwrap();

    // sync state
    let sync_details = client.sync_state().await.unwrap();

    // verify that the client is synced to the latest block
    assert_eq!(sync_details.block_num, rpc_api.blocks.last().unwrap().header().block_num());

    // verify that the latest block number has been updated
    assert_eq!(
        client.get_sync_height().await.unwrap(),
        rpc_api.blocks.last().unwrap().header().block_num()
    );

    // verify that we inserted the latest block into the DB via the client
    let latest_block = client.get_sync_height().await.unwrap();
    assert_eq!(sync_details.block_num, latest_block);
    assert_eq!(
        rpc_api.blocks.last().unwrap().hash(),
        client.test_store().get_block_headers(&[latest_block]).await.unwrap()[0]
            .0
            .hash()
    );

    // Try reconstructing the chain_mmr from what's in the database
    let partial_mmr = client.build_current_partial_mmr(true).await.unwrap();
    assert_eq!(partial_mmr.forest(), 6);
    assert!(partial_mmr.open(0).unwrap().is_some()); // Account anchor block
    assert!(partial_mmr.open(1).unwrap().is_some());
    assert!(partial_mmr.open(2).unwrap().is_none());
    assert!(partial_mmr.open(3).unwrap().is_none());
    assert!(partial_mmr.open(4).unwrap().is_some());
    assert!(partial_mmr.open(5).unwrap().is_none());

    // Ensure the proofs are valid
    let mmr_proof = partial_mmr.open(1).unwrap().unwrap();
    let (block_1, _) = rpc_api.get_block_header_by_number(Some(1), false).await.unwrap();
    partial_mmr.peaks().verify(block_1.hash(), mmr_proof).unwrap();

    let mmr_proof = partial_mmr.open(4).unwrap().unwrap();
    let (block_4, _) = rpc_api.get_block_header_by_number(Some(4), false).await.unwrap();
    partial_mmr.peaks().verify(block_4.hash(), mmr_proof).unwrap();
}

#[tokio::test]
async fn test_tags() {
    // generate test client with a random store name
    let (mut client, _rpc_api) = create_test_client().await;

    // Assert that the store gets created with the tag 0 (used for notes consumable by any account)
    assert!(client.get_note_tags().await.unwrap().is_empty());

    // add a tag
    let tag_1: NoteTag = 1.into();
    let tag_2: NoteTag = 2.into();
    client.add_note_tag(tag_1).await.unwrap();
    client.add_note_tag(tag_2).await.unwrap();

    // verify that the tag is being tracked
    assert_eq!(client.get_note_tags().await.unwrap(), vec![tag_1, tag_2]);

    // attempt to add the same tag again
    client.add_note_tag(tag_1).await.unwrap();

    // verify that the tag is still being tracked only once
    assert_eq!(client.get_note_tags().await.unwrap(), vec![tag_1, tag_2]);

    // Try removing non-existent tag
    let tag_4: NoteTag = 4.into();
    client.remove_note_tag(tag_4).await.unwrap();

    // verify that the tracked tags are unchanged
    assert_eq!(client.get_note_tags().await.unwrap(), vec![tag_1, tag_2]);

    // remove second tag
    client.remove_note_tag(tag_1).await.unwrap();

    // verify that tag_1 is not tracked anymore
    assert_eq!(client.get_note_tags().await.unwrap(), vec![tag_2]);
}

#[tokio::test]
async fn test_mint_transaction() {
    // generate test client with a random store name
    let (mut client, _rpc_api) = create_test_client().await;

    // Faucet account generation
    let (faucet, _seed) = insert_new_fungible_faucet(&mut client, AccountStorageMode::Private)
        .await
        .unwrap();

    client.sync_state().await.unwrap();

    // Test submitting a mint transaction
    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap(),
        miden_objects::notes::NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build();

    let transaction = client.new_transaction(faucet.id(), transaction_request).await.unwrap();

    assert!(transaction.executed_transaction().account_delta().nonce().is_some());
}

#[tokio::test]
async fn test_get_output_notes() {
    // generate test client with a random store name
    let (mut client, _rpc_api) = create_test_client().await;
    client.sync_state().await.unwrap();

    // Faucet account generation
    let (faucet, _seed) = insert_new_fungible_faucet(&mut client, AccountStorageMode::Private)
        .await
        .unwrap();

    // Test submitting a mint transaction
    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap(),
        miden_objects::notes::NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build();

    //Before executing transaction, there are no output notes
    assert!(client.get_output_notes(NoteFilter::All).await.unwrap().is_empty());

    let transaction = client.new_transaction(faucet.id(), transaction_request).await.unwrap();
    client.submit_transaction(transaction).await.unwrap();

    // Check that there was an output note but it wasn't consumed
    assert!(client.get_output_notes(NoteFilter::Consumed).await.unwrap().is_empty());
    assert!(!client.get_output_notes(NoteFilter::All).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_import_note_validation() {
    // generate test client
    let (mut client, rpc_api) = create_test_client().await;

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

#[tokio::test]
async fn test_transaction_request_expiration() {
    let (mut client, _) = create_test_client().await;
    client.sync_state().await.unwrap();

    let current_height = client.get_sync_height().await.unwrap();
    let (faucet, _seed) = insert_new_fungible_faucet(&mut client, AccountStorageMode::Private)
        .await
        .unwrap();

    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap(),
        miden_objects::notes::NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .with_expiration_delta(5)
    .unwrap()
    .build();

    let transaction = client.new_transaction(faucet.id(), transaction_request).await.unwrap();

    let (_, tx_outputs, ..) = transaction.executed_transaction().clone().into_parts();

    assert_eq!(tx_outputs.expiration_block_num, current_height + 5);
}

#[tokio::test]
async fn test_import_processing_note_returns_error() {
    // generate test client with a random store name
    let (mut client, _rpc_api) = create_test_client().await;
    client.sync_state().await.unwrap();

    let (account, _seed) =
        insert_new_wallet(&mut client, AccountStorageMode::Private).await.unwrap();

    // Faucet account generation
    let (faucet, _seed) = insert_new_fungible_faucet(&mut client, AccountStorageMode::Private)
        .await
        .unwrap();

    // Test submitting a mint transaction
    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        account.id(),
        miden_objects::notes::NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build();

    let transaction =
        client.new_transaction(faucet.id(), transaction_request.clone()).await.unwrap();
    client.submit_transaction(transaction).await.unwrap();

    let note_id = transaction_request.expected_output_notes().next().unwrap().id();
    let note = client.get_input_note(note_id).await.unwrap();

    let input = [(note.try_into().unwrap(), None)];
    let consume_note_request =
        TransactionRequestBuilder::new().with_unauthenticated_input_notes(input).build();
    let transaction = client
        .new_transaction(account.id(), consume_note_request.clone())
        .await
        .unwrap();
    client.submit_transaction(transaction.clone()).await.unwrap();

    let processing_notes = client.get_input_notes(NoteFilter::Processing).await.unwrap();

    assert!(matches!(
        client
            .import_note(NoteFile::NoteId(processing_notes[0].id()))
            .await
            .unwrap_err(),
        ClientError::NoteImportError { .. }
    ));
}

#[tokio::test]
async fn test_no_nonce_change_transaction_request() {
    let mut client = create_test_client().await.0;

    client.sync_state().await.unwrap();

    // Insert Account
    let (regular_account, _seed) =
        insert_new_wallet(&mut client, AccountStorageMode::Private).await.unwrap();

    // Prepare transaction

    let code = "
        begin
            push.1 push.2
            # => [1, 2]
            add push.3
            # => [1+2, 3]
            assert_eq
        end
        ";

    let tx_script = client.compile_tx_script(vec![], code).unwrap();

    let transaction_request =
        TransactionRequestBuilder::new().with_custom_script(tx_script).unwrap().build();

    let transaction_execution_result =
        client.new_transaction(regular_account.id(), transaction_request).await.unwrap();

    let result = client.testing_apply_transaction(transaction_execution_result).await;

    assert!(matches!(
        result,
        Err(ClientError::StoreError(StoreError::AccountHashAlreadyExists(_)))
    ));
}

#[tokio::test]
async fn test_note_without_asset() {
    let (mut client, _rpc_api) = create_test_client().await;

    let (faucet, _seed) = insert_new_fungible_faucet(&mut client, AccountStorageMode::Private)
        .await
        .unwrap();

    let (wallet, _seed) =
        insert_new_wallet(&mut client, AccountStorageMode::Private).await.unwrap();

    client.sync_state().await.unwrap();

    // Create note without assets
    let serial_num = client.rng().draw_word();
    let recipient = utils::build_p2id_recipient(wallet.id(), serial_num).unwrap();
    let tag = NoteTag::from_account_id(wallet.id(), NoteExecutionMode::Local).unwrap();
    let metadata =
        NoteMetadata::new(wallet.id(), NoteType::Private, tag, NoteExecutionHint::always(), ZERO)
            .unwrap();
    let vault = NoteAssets::new(vec![]).unwrap();

    let note = Note::new(vault.clone(), metadata, recipient.clone());

    // Create and execute transaction
    let transaction_request = TransactionRequestBuilder::new()
        .with_own_output_notes(vec![OutputNote::Full(note)])
        .unwrap()
        .build();

    let transaction = client.new_transaction(wallet.id(), transaction_request.clone()).await;

    assert!(transaction.is_ok());

    // Create the same transaction for the faucet
    let metadata =
        NoteMetadata::new(faucet.id(), NoteType::Private, tag, NoteExecutionHint::always(), ZERO)
            .unwrap();
    let note = Note::new(vault, metadata, recipient);

    let transaction_request = TransactionRequestBuilder::new()
        .with_own_output_notes(vec![OutputNote::Full(note)])
        .unwrap()
        .build();

    let error = client.new_transaction(faucet.id(), transaction_request).await.unwrap_err();

    assert!(matches!(
        error,
        ClientError::TransactionRequestError(
            TransactionRequestError::TransactionScriptBuilderError(
                TransactionScriptBuilderError::FaucetNoteWithoutAsset
            )
        )
    ));
}
