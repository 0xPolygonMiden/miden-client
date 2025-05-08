use alloc::vec::Vec;
use std::collections::BTreeSet;

// TESTS
// ================================================================================================
use miden_lib::{
    account::{
        auth::RpoFalcon512, faucets::BasicFungibleFaucet, interface::AccountInterfaceError,
        wallets::BasicWallet,
    },
    note::utils,
    transaction::TransactionKernel,
};
use miden_objects::{
    Felt, FieldElement, Word, ZERO,
    account::{
        Account, AccountBuilder, AccountCode, AccountHeader, AccountId, AccountStorageMode,
        AccountType, AuthSecretKey,
    },
    asset::{Asset, FungibleAsset, TokenSymbol},
    crypto::{dsa::rpo_falcon512::SecretKey, rand::FeltRng},
    note::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteFile, NoteMetadata, NoteTag,
        NoteType,
    },
    testing::account_id::{
        ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1, ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_2,
        ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE,
        ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE,
        ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_UPDATABLE_CODE,
    },
    transaction::OutputNote,
    vm::AdviceInputs,
};
use miden_tx::utils::{Deserializable, Serializable};
use rand::{RngCore, rngs::StdRng};

use crate::{
    Client, ClientError,
    keystore::FilesystemKeyStore,
    mock::create_test_client,
    rpc::NodeRpcClient,
    store::{InputNoteRecord, NoteFilter, Store, StoreError},
    transaction::{PaymentTransactionData, TransactionRequestBuilder, TransactionRequestError},
};

async fn insert_new_wallet(
    client: &mut Client,
    storage_mode: AccountStorageMode,
    keystore: &FilesystemKeyStore<StdRng>,
) -> Result<(Account, Word), ClientError> {
    let key_pair = SecretKey::with_rng(&mut client.rng);
    let pub_key = key_pair.public_key();

    keystore.add_key(&AuthSecretKey::RpoFalcon512(key_pair)).unwrap();

    let mut init_seed = [0u8; 32];
    client.rng.fill_bytes(&mut init_seed);

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

    Ok((account, seed))
}

async fn insert_new_fungible_faucet(
    client: &mut Client,
    storage_mode: AccountStorageMode,
    keystore: &FilesystemKeyStore<StdRng>,
) -> Result<(Account, Word), ClientError> {
    let key_pair = SecretKey::with_rng(&mut client.rng);
    let pub_key = key_pair.public_key();

    keystore.add_key(&AuthSecretKey::RpoFalcon512(key_pair)).unwrap();

    // we need to use an initial seed to create the wallet account
    let mut init_seed = [0u8; 32];
    client.rng.fill_bytes(&mut init_seed);

    let symbol = TokenSymbol::new("TEST").unwrap();
    let max_supply = Felt::try_from(9_999_999_u64.to_le_bytes().as_slice())
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
    Ok((account, seed))
}

#[tokio::test]
async fn test_input_notes_round_trip() {
    // generate test client with a random store name
    let (mut client, rpc_api, keystore) = create_test_client().await;

    insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore)
        .await
        .unwrap();
    // generate test data
    let available_notes = [rpc_api.get_note_at(0), rpc_api.get_note_at(1)];

    // insert notes into database
    for note in &available_notes {
        client
            .import_note(NoteFile::NoteWithProof(
                note.note().clone(),
                note.proof().expect("These notes should be authenticated").clone(),
            ))
            .await
            .unwrap();
    }

    // retrieve notes from database
    let retrieved_notes = client.get_input_notes(NoteFilter::Unverified).await.unwrap();
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
    let (mut client, rpc_api, _) = create_test_client().await;
    // Get note from mocked RPC backend since any note works here
    let original_note = rpc_api.get_note_at(0).note().clone();

    // insert Note into database
    let note: InputNoteRecord = original_note.clone().into();
    client
        .import_note(NoteFile::NoteDetails {
            details: note.into(),
            tag: None,
            after_block_num: 0.into(),
        })
        .await
        .unwrap();

    // retrieve note from database
    let retrieved_note = client.get_input_note(original_note.id()).await.unwrap().unwrap();

    let recorded_note: InputNoteRecord = original_note.into();
    assert_eq!(recorded_note.id(), retrieved_note.id());
}

#[tokio::test]
async fn insert_basic_account() {
    // generate test client with a random store name
    let (mut client, _rpc_api, keystore) = create_test_client().await;

    // Insert Account
    let account_insert_result =
        insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore).await;
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account(account.id()).await;
    assert!(fetched_account_data.is_ok());

    let fetched_account = fetched_account_data.unwrap().unwrap();
    let fetched_account_seed = fetched_account.seed().copied();
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
    let (mut client, _rpc_api, keystore) = create_test_client().await;

    // Insert Account
    let account_insert_result =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &keystore).await;
    assert!(account_insert_result.is_ok());

    let (account, account_seed) = account_insert_result.unwrap();

    // Fetch Account
    let fetched_account_data = client.get_account(account.id()).await;
    assert!(fetched_account_data.is_ok());

    let fetched_account = fetched_account_data.unwrap().unwrap();
    let fetched_account_seed = fetched_account.seed().copied();
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
    let (mut client, _rpc_api, _) = create_test_client().await;

    let account = Account::mock(
        ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_2,
        Felt::new(2),
        TransactionKernel::testing_assembler(),
    );

    assert!(client.add_account(&account, Some(Word::default()), false).await.is_ok());
    assert!(client.add_account(&account, Some(Word::default()), false).await.is_err());
}

#[tokio::test]
async fn test_account_code() {
    // generate test client with a random store name
    let (mut client, _rpc_api, _) = create_test_client().await;

    let account = Account::mock(
        ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE,
        Felt::ZERO,
        TransactionKernel::testing_assembler(),
    );

    let account_code = account.code();

    let account_code_bytes = account_code.to_bytes();

    let reconstructed_code = AccountCode::read_from_bytes(&account_code_bytes).unwrap();
    assert_eq!(*account_code, reconstructed_code);

    client.add_account(&account, Some(Word::default()), false).await.unwrap();
    let retrieved_acc = client.get_account(account.id()).await.unwrap().unwrap();
    assert_eq!(*account.code(), *retrieved_acc.account().code());
}

#[tokio::test]
async fn test_get_account_by_id() {
    // generate test client with a random store name
    let (mut client, _rpc_api, _) = create_test_client().await;

    let account = Account::mock(
        ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_UPDATABLE_CODE,
        Felt::new(10),
        TransactionKernel::assembler(),
    );

    client.add_account(&account, Some(Word::default()), false).await.unwrap();

    // Retrieving an existing account should succeed
    let (acc_from_db, _account_seed) = match client.get_account_header_by_id(account.id()).await {
        Ok(account) => account.unwrap(),
        Err(err) => panic!("Error retrieving account: {err}"),
    };
    assert_eq!(AccountHeader::from(account), acc_from_db);

    // Retrieving a non existing account should fail
    let invalid_id = AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_2).unwrap();
    assert!(client.get_account_header_by_id(invalid_id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_sync_state() {
    // generate test client with a random store name
    let (mut client, rpc_api, _) = create_test_client().await;

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
    // TODO: Review these next 3 asserts (see PR 758)
    assert_eq!(client.get_input_notes(NoteFilter::Committed).await.unwrap().len(), 2);
    assert_eq!(client.get_input_notes(NoteFilter::Consumed).await.unwrap().len(), 0);
    assert_eq!(sync_details.consumed_notes.len(), 0);

    // verify that the latest block number has been updated
    assert_eq!(
        client.get_sync_height().await.unwrap(),
        rpc_api.blocks.last().unwrap().header().block_num()
    );
}

#[tokio::test]
async fn test_sync_state_mmr() {
    // generate test client with a random store name
    let (mut client, rpc_api, keystore) = create_test_client().await;
    // Import note and create wallet so that synced notes do not get discarded (due to being
    // irrelevant)
    insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore)
        .await
        .unwrap();

    for note in rpc_api.notes.values() {
        let note_file = NoteFile::NoteDetails {
            details: note.note().clone().into(),
            after_block_num: 0.into(),
            tag: Some(note.note().metadata().tag()),
        };

        client.import_note(note_file).await.unwrap();
    }

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
        rpc_api.blocks.last().unwrap().commitment(),
        client
            .test_store()
            .get_block_headers(&[latest_block].into_iter().collect())
            .await
            .unwrap()[0]
            .0
            .commitment()
    );

    // Try reconstructing the partial_mmr from what's in the database
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
    let (block_1, _) = rpc_api.get_block_header_by_number(Some(1.into()), false).await.unwrap();
    partial_mmr.peaks().verify(block_1.commitment(), mmr_proof).unwrap();

    let mmr_proof = partial_mmr.open(4).unwrap().unwrap();
    let (block_4, _) = rpc_api.get_block_header_by_number(Some(4.into()), false).await.unwrap();
    partial_mmr.peaks().verify(block_4.commitment(), mmr_proof).unwrap();
}

#[tokio::test]
async fn test_tags() {
    // generate test client with a random store name
    let (mut client, _rpc_api, _) = create_test_client().await;

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
    let (mut client, _rpc_api, keystore) = create_test_client().await;

    // Faucet account generation
    let (faucet, _seed) =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &keystore)
            .await
            .unwrap();

    client.sync_state().await.unwrap();

    // Test submitting a mint transaction
    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET_1).unwrap(),
        miden_objects::note::NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    let transaction = client.new_transaction(faucet.id(), transaction_request).await.unwrap();

    assert!(transaction.executed_transaction().account_delta().nonce().is_some());
}

#[tokio::test]
async fn test_get_output_notes() {
    // generate test client with a random store name
    let (mut client, _rpc_api, keystore) = create_test_client().await;
    client.sync_state().await.unwrap();

    // Faucet account generation
    let (faucet, _seed) =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &keystore)
            .await
            .unwrap();

    // Test submitting a mint transaction
    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::try_from(ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE).unwrap(),
        miden_objects::note::NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

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
    let (mut client, rpc_api, _) = create_test_client().await;

    // generate test data
    let committed_note: InputNoteRecord = rpc_api.get_note_at(0).into();
    let expected_note: InputNoteRecord = rpc_api.get_note_at(1).note().clone().into();

    client
        .import_note(NoteFile::NoteDetails {
            details: committed_note.clone().into(),
            after_block_num: 0.into(),
            tag: None,
        })
        .await
        .unwrap();
    assert!(client.import_note(NoteFile::NoteId(expected_note.id())).await.is_err());
    client
        .import_note(NoteFile::NoteDetails {
            details: expected_note.clone().into(),
            after_block_num: 0.into(),
            tag: None,
        })
        .await
        .unwrap();

    assert!(expected_note.inclusion_proof().is_none());
    assert!(committed_note.inclusion_proof().is_some());
}

#[tokio::test]
async fn test_transaction_request_expiration() {
    let (mut client, _, keystore) = create_test_client().await;
    client.sync_state().await.unwrap();

    let current_height = client.get_sync_height().await.unwrap();
    let (faucet, _seed) =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &keystore)
            .await
            .unwrap();

    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        AccountId::try_from(ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE).unwrap(),
        miden_objects::note::NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .with_expiration_delta(5)
    .build()
    .unwrap();

    let transaction = client.new_transaction(faucet.id(), transaction_request).await.unwrap();

    let (_, tx_outputs, ..) = transaction.executed_transaction().clone().into_parts();

    assert_eq!(tx_outputs.expiration_block_num, current_height + 5);
}

#[tokio::test]
async fn test_import_processing_note_returns_error() {
    // generate test client with a random store name
    let (mut client, _rpc_api, keystore) = create_test_client().await;
    client.sync_state().await.unwrap();

    let (account, _seed) = insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore)
        .await
        .unwrap();

    // Faucet account generation
    let (faucet, _seed) =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &keystore)
            .await
            .unwrap();

    // Test submitting a mint transaction
    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        account.id(),
        miden_objects::note::NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    let transaction =
        client.new_transaction(faucet.id(), transaction_request.clone()).await.unwrap();
    client.submit_transaction(transaction).await.unwrap();

    let note_id = transaction_request.expected_output_notes().next().unwrap().id();
    let note = client.get_input_note(note_id).await.unwrap().unwrap();

    let input = [(note.try_into().unwrap(), None)];
    let consume_note_request = TransactionRequestBuilder::new()
        .with_unauthenticated_input_notes(input)
        .build()
        .unwrap();
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
    let (mut client, _, keystore) = create_test_client().await;

    client.sync_state().await.unwrap();

    // Insert Account
    let (regular_account, _seed) =
        insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore)
            .await
            .unwrap();

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
        TransactionRequestBuilder::new().with_custom_script(tx_script).build().unwrap();

    let transaction_execution_result =
        client.new_transaction(regular_account.id(), transaction_request).await.unwrap();

    let result = client.testing_apply_transaction(transaction_execution_result).await;

    assert!(matches!(
        result,
        Err(ClientError::StoreError(StoreError::AccountCommitmentAlreadyExists(_)))
    ));
}

#[tokio::test]
async fn test_note_without_asset() {
    let (mut client, _rpc_api, keystore) = create_test_client().await;

    let (faucet, _seed) =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &keystore)
            .await
            .unwrap();

    let (wallet, _seed) = insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore)
        .await
        .unwrap();

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
        .build()
        .unwrap();

    let transaction = client.new_transaction(wallet.id(), transaction_request.clone()).await;

    assert!(transaction.is_ok());

    // Create the same transaction for the faucet
    let metadata =
        NoteMetadata::new(faucet.id(), NoteType::Private, tag, NoteExecutionHint::always(), ZERO)
            .unwrap();
    let note = Note::new(vault, metadata, recipient);

    let transaction_request = TransactionRequestBuilder::new()
        .with_own_output_notes(vec![OutputNote::Full(note)])
        .build()
        .unwrap();

    let error = client.new_transaction(faucet.id(), transaction_request).await.unwrap_err();

    assert!(matches!(
        error,
        ClientError::TransactionRequestError(TransactionRequestError::AccountInterfaceError(
            AccountInterfaceError::FaucetNoteWithoutAsset
        ))
    ));

    let error = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![], faucet.id(), wallet.id()),
        None,
        NoteType::Public,
        client.rng(),
    )
    .unwrap_err();

    assert!(matches!(error, TransactionRequestError::P2IDNoteWithoutAsset));

    let error = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(
            vec![Asset::Fungible(FungibleAsset::new(faucet.id(), 0).unwrap())],
            faucet.id(),
            wallet.id(),
        ),
        None,
        NoteType::Public,
        client.rng(),
    )
    .unwrap_err();

    assert!(matches!(error, TransactionRequestError::P2IDNoteWithoutAsset));
}

#[tokio::test]
async fn test_execute_program() {
    let (mut client, _, keystore) = create_test_client().await;

    let (wallet, _seed) = insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore)
        .await
        .unwrap();

    let code = "
        use.std::sys

        begin
            push.16
            repeat.16
                dup push.1 sub
            end
            exec.sys::truncate_stack
        end
        ";

    let tx_script = client.compile_tx_script(vec![], code).unwrap();

    let output_stack = client
        .execute_program(wallet.id(), tx_script, AdviceInputs::default(), BTreeSet::new())
        .await
        .unwrap();

    let mut expected_stack = [Felt::new(0); 16];
    for (i, element) in expected_stack.iter_mut().enumerate() {
        *element = Felt::new(i as u64);
    }

    assert_eq!(output_stack, expected_stack);
}
