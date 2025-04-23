use alloc::vec::Vec;
use std::{boxed::Box, collections::BTreeSet, env::temp_dir, println, sync::Arc};

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
    crypto::{
        dsa::rpo_falcon512::SecretKey,
        rand::{FeltRng, RpoRandomCoin},
    },
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
    transaction::{InputNote, OutputNote},
    vm::AdviceInputs,
};
use miden_tx::{
    TransactionExecutorError,
    utils::{Deserializable, Serializable},
};
use rand::{Rng, RngCore, rngs::StdRng};
use uuid::Uuid;

use crate::{
    Client, ClientError,
    keystore::FilesystemKeyStore,
    note::NoteRelevance,
    rpc::NodeRpcClient,
    store::{
        InputNoteRecord, InputNoteState, NoteFilter, StoreError, TransactionFilter,
        input_note_states::ConsumedAuthenticatedLocalNoteState, sqlite_store::SqliteStore,
    },
    sync::{NoteTagSource, TX_GRACEFUL_BLOCKS},
    testing::{
        common::{
            ACCOUNT_ID_REGULAR, MINT_AMOUNT, RECALL_HEIGHT_DELTA, TRANSFER_AMOUNT,
            assert_account_has_single_asset, assert_note_cannot_be_consumed_twice, consume_notes,
            execute_failing_tx, execute_tx, execute_tx_and_sync, mint_and_consume, mint_note,
            setup_two_wallets_and_faucet, setup_wallet_and_faucet, wait_for_node, wait_for_tx,
        },
        mock::{MockClient, MockRpcApi},
    },
    transaction::{
        PaymentTransactionData, TransactionRequestBuilder, TransactionRequestError,
        TransactionStatus,
    },
};

// HELPERS
// ================================================================================================

pub async fn create_test_client() -> (MockClient, MockRpcApi, FilesystemKeyStore<StdRng>) {
    let store = SqliteStore::new(create_test_store_path()).await.unwrap();
    let store = Arc::new(store);

    let mut rng = rand::rng();
    let coin_seed: [u64; 4] = rng.random();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));

    let keystore = FilesystemKeyStore::new(temp_dir()).unwrap();

    let rpc_api = MockRpcApi::new();
    let arc_rpc_api = Arc::new(rpc_api.clone());

    let client =
        MockClient::new(arc_rpc_api, Box::new(rng), store, Arc::new(keystore.clone()), true);
    (client, rpc_api, keystore)
}

pub fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}

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

// TESTS
// ================================================================================================

#[tokio::test]
async fn test_input_notes_round_trip() {
    // generate test client with a random store name
    let (mut client, rpc_api, keystore) = create_test_client().await;

    insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore)
        .await
        .unwrap();
    // generate test data
    let available_notes = rpc_api.get_available_notes();

    // insert notes into database
    for note in &available_notes {
        client
            .import_note(NoteFile::NoteWithProof(
                note.note().unwrap().clone(),
                note.inclusion_proof().clone(),
            ))
            .await
            .unwrap();
    }

    // retrieve notes from database
    assert_eq!(client.get_input_notes(NoteFilter::Unverified).await.unwrap().len(), 1);
    assert_eq!(client.get_input_notes(NoteFilter::Consumed).await.unwrap().len(), 1);

    let retrieved_notes = client.get_input_notes(NoteFilter::All).await.unwrap();
    assert_eq!(retrieved_notes.len(), 2);

    let recorded_notes: Vec<InputNoteRecord> = available_notes
        .into_iter()
        .map(|n| {
            let input_note: InputNote = n.try_into().unwrap();
            input_note.into()
        })
        .collect();
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
    let original_note = rpc_api.get_available_notes()[0].note().unwrap().clone();

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
    let expected_notes = rpc_api
        .get_available_notes()
        .into_iter()
        .map(|n| n.note().unwrap().clone())
        .collect::<Vec<Note>>();

    for note in &expected_notes {
        client
            .import_note(NoteFile::NoteDetails {
                details: note.clone().into(),
                after_block_num: 0.into(),
                tag: Some(note.metadata().tag()),
            })
            .await
            .unwrap();
    }

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
    assert_eq!(sync_details.block_num, rpc_api.get_chain_tip_block_num());

    // verify that we now have one committed note after syncing state
    // TODO: Review these next 3 asserts (see PR 758)
    assert_eq!(client.get_input_notes(NoteFilter::Committed).await.unwrap().len(), 1);
    assert_eq!(client.get_input_notes(NoteFilter::Consumed).await.unwrap().len(), 1);
    assert_eq!(sync_details.consumed_notes.len(), 1);

    // verify that the latest block number has been updated
    assert_eq!(client.get_sync_height().await.unwrap(), rpc_api.get_chain_tip_block_num());
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

    let notes = rpc_api
        .get_available_notes()
        .into_iter()
        .map(|n| n.note().unwrap().clone())
        .collect::<Vec<Note>>();

    for note in &notes {
        client
            .import_note(NoteFile::NoteDetails {
                details: note.clone().into(),
                after_block_num: 0.into(),
                tag: Some(note.metadata().tag()),
            })
            .await
            .unwrap();
    }

    // sync state
    let sync_details = client.sync_state().await.unwrap();

    // verify that the client is synced to the latest block
    assert_eq!(sync_details.block_num, rpc_api.get_chain_tip_block_num());

    // verify that the latest block number has been updated
    assert_eq!(client.get_sync_height().await.unwrap(), rpc_api.get_chain_tip_block_num());

    // verify that we inserted the latest block into the DB via the client
    let latest_block = client.get_sync_height().await.unwrap();
    assert_eq!(sync_details.block_num, latest_block);
    assert_eq!(
        rpc_api.get_block_header_by_number(None, false).await.unwrap().0.commitment(),
        client
            .test_store()
            .get_block_headers(&[latest_block].into_iter().collect())
            .await
            .unwrap()[0]
            .0
            .commitment()
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
async fn test_import_note_validation() {
    // generate test client
    let (mut client, rpc_api, _) = create_test_client().await;

    // generate test data
    let consumed_note = rpc_api.get_available_notes()[0].clone();
    let expected_note = rpc_api.get_available_notes()[1].clone();

    client
        .import_note(NoteFile::NoteWithProof(
            consumed_note.note().unwrap().clone(),
            consumed_note.inclusion_proof().clone(),
        ))
        .await
        .unwrap();

    client
        .import_note(NoteFile::NoteDetails {
            details: expected_note.note().unwrap().into(),
            after_block_num: 0.into(),
            tag: None,
        })
        .await
        .unwrap();

    let expected_note = client
        .get_input_note(expected_note.note().unwrap().id())
        .await
        .unwrap()
        .unwrap();

    let consumed_note = client
        .get_input_note(consumed_note.note().unwrap().id())
        .await
        .unwrap()
        .unwrap();

    assert!(expected_note.inclusion_proof().is_none());
    assert!(consumed_note.is_consumed());
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
        miden_objects::note::NoteType::Public,
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

#[tokio::test]
async fn test_real_note_roundtrip() {
    let (mut client, _, keystore) = create_test_client().await;
    let (wallet, _seed) = insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore)
        .await
        .unwrap();
    let (faucet, _seed) =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &keystore)
            .await
            .unwrap();

    client.sync_state().await.unwrap();

    // Test submitting a mint transaction
    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet.id(), 5u64).unwrap(),
        wallet.id(),
        miden_objects::note::NoteType::Public,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    let note_id = transaction_request.expected_output_notes().next().unwrap().id();
    let transaction = client.new_transaction(faucet.id(), transaction_request).await.unwrap();
    client.submit_transaction(transaction).await.unwrap();

    let note = client.get_input_note(note_id).await.unwrap().unwrap();
    assert!(matches!(note.state(), &InputNoteState::Expected(_)));

    client.sync_state().await.unwrap();

    let note = client.get_input_note(note_id).await.unwrap().unwrap();
    assert!(matches!(note.state(), &InputNoteState::Committed(_)));

    // Consume note
    let transaction_request =
        TransactionRequestBuilder::consume_notes(vec![note_id]).build().unwrap();

    let transaction = client.new_transaction(wallet.id(), transaction_request).await.unwrap();
    client.submit_transaction(transaction).await.unwrap();

    client.sync_state().await.unwrap();

    let note = client.get_input_note(note_id).await.unwrap().unwrap();
    assert!(matches!(note.state(), &InputNoteState::ConsumedAuthenticatedLocal(_)));
}

#[tokio::test]
async fn test_added_notes() {
    let (mut client, _, authenticator) = create_test_client().await;
    wait_for_node(&mut client).await;

    let faucet_account_header =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap()
            .0;

    // Mint some asset for an account not tracked by the client. It should not be stored as an
    // input note afterwards since it is not being tracked by the client
    let fungible_asset = FungibleAsset::new(faucet_account_header.id(), MINT_AMOUNT).unwrap();
    let tx_request = TransactionRequestBuilder::mint_fungible_asset(
        fungible_asset,
        AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap(),
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    println!("Running Mint tx...");
    execute_tx_and_sync(&mut client, faucet_account_header.id(), tx_request).await;

    // Check that no new notes were added
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).await.unwrap();
    assert!(notes.is_empty());
}

#[tokio::test]
async fn test_p2id_transfer() {
    let (mut client, _, authenticator) = create_test_client().await;
    wait_for_node(&mut client).await;

    let (first_regular_account, second_regular_account, faucet_account_header) =
        setup_two_wallets_and_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    // First Mint necessary token
    mint_and_consume(&mut client, from_account_id, faucet_account_id, NoteType::Private).await;
    assert_account_has_single_asset(&client, from_account_id, faucet_account_id, MINT_AMOUNT).await;

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    println!("Running P2ID tx...");
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        None,
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    let note = tx_request.expected_output_notes().next().unwrap().clone();
    let transaction_id = execute_tx(&mut client, from_account_id, tx_request).await;

    // Check that a note tag started being tracked for this note.
    assert!(
        client
            .get_note_tags()
            .await
            .unwrap()
            .into_iter()
            .any(|tag| tag.source == NoteTagSource::Note(note.id()))
    );

    wait_for_tx(&mut client, transaction_id).await;

    // Check that the tag is not longer being tracked
    assert!(
        !client
            .get_note_tags()
            .await
            .unwrap()
            .into_iter()
            .any(|tag| tag.source == NoteTagSource::Note(note.id()))
    );

    // Check that note is committed for the second account to consume
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).await.unwrap();
    assert!(!notes.is_empty());

    // Consume P2ID note
    println!("Consuming Note...");
    let tx_request = TransactionRequestBuilder::consume_notes(vec![notes[0].id()]).build().unwrap();
    execute_tx_and_sync(&mut client, to_account_id, tx_request).await;

    // Ensure we have nothing else to consume
    let current_notes = client.get_input_notes(NoteFilter::Committed).await.unwrap();
    assert!(current_notes.is_empty());

    let regular_account = client.get_account(from_account_id).await.unwrap().unwrap();
    let seed = regular_account.seed().copied();
    let regular_account: Account = regular_account.into();

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

    let regular_account: Account = client.get_account(to_account_id).await.unwrap().unwrap().into();
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
async fn test_p2id_transfer_failing_not_enough_balance() {
    let (mut client, _, authenticator) = create_test_client().await;
    wait_for_node(&mut client).await;

    let (first_regular_account, second_regular_account, faucet_account_header) =
        setup_two_wallets_and_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    // First Mint necessary token
    mint_and_consume(&mut client, from_account_id, faucet_account_id, NoteType::Private).await;

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT + 1).unwrap();
    println!("Running P2ID tx...");
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        None,
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    execute_failing_tx(
        &mut client,
        from_account_id,
        tx_request,
        ClientError::AssetError(miden_objects::AssetError::FungibleAssetAmountNotSufficient {
            minuend: MINT_AMOUNT,
            subtrahend: MINT_AMOUNT + 1,
        }),
    )
    .await;
}

#[tokio::test]
async fn test_p2idr_transfer_consumed_by_target() {
    let (mut client, _, authenticator) = create_test_client().await;
    wait_for_node(&mut client).await;

    let (first_regular_account, second_regular_account, faucet_account_header) =
        setup_two_wallets_and_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    // First Mint necessary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::Private).await;
    println!("about to consume");

    //Check that the note is not consumed by the target account
    assert!(matches!(
        client.get_input_note(note.id()).await.unwrap().unwrap().state(),
        InputNoteState::Committed { .. }
    ));

    consume_notes(&mut client, from_account_id, &[note.clone()]).await;
    assert_account_has_single_asset(&client, from_account_id, faucet_account_id, MINT_AMOUNT).await;

    // Check that the note is consumed by the target account
    let input_note = client.get_input_note(note.id()).await.unwrap().unwrap();
    assert!(matches!(input_note.state(), InputNoteState::ConsumedAuthenticatedLocal { .. }));
    if let InputNoteState::ConsumedAuthenticatedLocal(ConsumedAuthenticatedLocalNoteState {
        submission_data,
        ..
    }) = input_note.state()
    {
        assert_eq!(submission_data.consumer_account, from_account_id);
    } else {
        panic!("Note should be consumed");
    }

    // Do a transfer from first account to second account with Recall. In this situation we'll do
    // the happy path where the `to_account_id` consumes the note
    println!("getting balance");
    let from_account_balance = client
        .get_account(from_account_id)
        .await
        .unwrap()
        .unwrap()
        .account()
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let to_account_balance = client
        .get_account(to_account_id)
        .await
        .unwrap()
        .unwrap()
        .account()
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let current_block_num = client.get_sync_height().await.unwrap();
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    println!("Running P2IDR tx...");
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        Some(current_block_num + RECALL_HEIGHT_DELTA),
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    execute_tx_and_sync(&mut client, from_account_id, tx_request.clone()).await;

    // Check that note is committed for the second account to consume
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).await.unwrap();
    assert!(!notes.is_empty());

    // Make the `to_account_id` consume P2IDR note
    let note_id = tx_request.expected_output_notes().next().unwrap().id();
    println!("Consuming Note...");
    let tx_request = TransactionRequestBuilder::consume_notes(vec![note_id]).build().unwrap();
    execute_tx_and_sync(&mut client, to_account_id, tx_request).await;
    let regular_account = client.get_account(from_account_id).await.unwrap().unwrap();

    // The seed should not be retrieved due to the account not being new
    assert!(!regular_account.account().is_new() && regular_account.seed().is_none());
    assert_eq!(regular_account.account().vault().assets().count(), 1);
    let asset = regular_account.account().vault().assets().next().unwrap();

    // Validate the transfered amounts
    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), from_account_balance - TRANSFER_AMOUNT);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    let regular_account: Account = client.get_account(to_account_id).await.unwrap().unwrap().into();
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), to_account_balance + TRANSFER_AMOUNT);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    assert_note_cannot_be_consumed_twice(&mut client, to_account_id, note_id).await;
}

#[tokio::test]
async fn test_p2idr_transfer_consumed_by_sender() {
    let (mut client, mock_rpc_api, authenticator) = create_test_client().await;
    wait_for_node(&mut client).await;

    let (first_regular_account, second_regular_account, faucet_account_header) =
        setup_two_wallets_and_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    // First Mint necessary token
    mint_and_consume(&mut client, from_account_id, faucet_account_id, NoteType::Private).await;

    // Do a transfer from first account to second account with Recall. In this situation we'll do
    // the happy path where the `to_account_id` consumes the note
    let from_account_balance = client
        .get_account(from_account_id)
        .await
        .unwrap()
        .unwrap()
        .account()
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let current_block_num = client.get_sync_height().await.unwrap();
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    println!("Running P2IDR tx...");
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        Some(current_block_num + RECALL_HEIGHT_DELTA),
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    execute_tx_and_sync(&mut client, from_account_id, tx_request).await;

    // Check that note is committed
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).await.unwrap();
    assert!(!notes.is_empty());

    // Check that it's still too early to consume
    println!("Consuming Note (too early)...");
    let tx_request = TransactionRequestBuilder::consume_notes(vec![notes[0].id()]).build().unwrap();
    let transaction_execution_result = client.new_transaction(from_account_id, tx_request).await;
    assert!(transaction_execution_result.is_err_and(|err| {
        matches!(
            err,
            ClientError::TransactionExecutorError(
                TransactionExecutorError::TransactionProgramExecutionFailed(_)
            )
        )
    }));

    // Wait to consume with the sender account
    println!("Waiting for note to be consumable by sender");
    mock_rpc_api.advance_blocks(RECALL_HEIGHT_DELTA);
    client.sync_state().await.unwrap();

    // Consume the note with the sender account
    println!("Consuming Note...");
    let tx_request = TransactionRequestBuilder::consume_notes(vec![notes[0].id()]).build().unwrap();
    execute_tx_and_sync(&mut client, from_account_id, tx_request).await;

    let regular_account = client.get_account(from_account_id).await.unwrap().unwrap();
    // The seed should not be retrieved due to the account not being new
    assert!(!regular_account.account().is_new() && regular_account.seed().is_none());
    assert_eq!(regular_account.account().vault().assets().count(), 1);
    let asset = regular_account.account().vault().assets().next().unwrap();

    // Validate the sender hasn't lost funds
    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), from_account_balance);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    let regular_account: Account = client.get_account(to_account_id).await.unwrap().unwrap().into();
    assert_eq!(regular_account.vault().assets().count(), 0);

    // Check that the target can't consume the note anymore
    assert_note_cannot_be_consumed_twice(&mut client, to_account_id, notes[0].id()).await;
}

#[tokio::test]
async fn test_get_consumable_notes() {
    let (mut client, _, authenticator) = create_test_client().await;

    let (first_regular_account, second_regular_account, faucet_account_header) =
        setup_two_wallets_and_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    //No consumable notes initially
    assert!(client.get_consumable_notes(None).await.unwrap().is_empty());

    // First Mint necessary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::Private).await;

    // Check that note is consumable by the account that minted
    assert!(!client.get_consumable_notes(None).await.unwrap().is_empty());
    assert!(!client.get_consumable_notes(Some(from_account_id)).await.unwrap().is_empty());
    assert!(client.get_consumable_notes(Some(to_account_id)).await.unwrap().is_empty());

    consume_notes(&mut client, from_account_id, &[note]).await;

    //After consuming there are no more consumable notes
    assert!(client.get_consumable_notes(None).await.unwrap().is_empty());

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    println!("Running P2IDR tx...");
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        Some(100.into()),
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    execute_tx_and_sync(&mut client, from_account_id, tx_request).await;

    // Check that note is consumable by both accounts
    let consumable_notes = client.get_consumable_notes(None).await.unwrap();
    let relevant_accounts = &consumable_notes.first().unwrap().1;
    assert_eq!(relevant_accounts.len(), 2);
    assert!(!client.get_consumable_notes(Some(from_account_id)).await.unwrap().is_empty());
    assert!(!client.get_consumable_notes(Some(to_account_id)).await.unwrap().is_empty());

    // Check that the note is only consumable after block 100 for the account that sent the
    // transaction
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
    let (mut client, _, authenticator) = create_test_client().await;

    let (first_regular_account, faucet_account_header) =
        setup_wallet_and_faucet(&mut client, AccountStorageMode::Private, &authenticator).await;

    let from_account_id = first_regular_account.id();
    let faucet_account_id = faucet_account_header.id();
    let random_account_id = AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap();

    // No output notes initially
    assert!(client.get_output_notes(NoteFilter::All).await.unwrap().is_empty());

    // First Mint necessary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::Private).await;

    // Check that there was an output note but it wasn't consumed
    assert!(client.get_output_notes(NoteFilter::Consumed).await.unwrap().is_empty());
    assert!(!client.get_output_notes(NoteFilter::All).await.unwrap().is_empty());

    consume_notes(&mut client, from_account_id, &[note]).await;

    //After consuming, the note is returned when using the [NoteFilter::Consumed] filter
    assert!(!client.get_output_notes(NoteFilter::Consumed).await.unwrap().is_empty());

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    println!("Running P2ID tx...");
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(
            vec![Asset::Fungible(asset)],
            from_account_id,
            random_account_id,
        ),
        None,
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    let output_note_id = tx_request.expected_output_notes().next().unwrap().id();

    // Before executing, the output note is not found
    assert!(client.get_output_note(output_note_id).await.unwrap().is_none());

    execute_tx_and_sync(&mut client, from_account_id, tx_request).await;

    // After executing, the note is only found in output notes
    assert!(client.get_output_note(output_note_id).await.unwrap().is_some());
    assert!(client.get_input_note(output_note_id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_stale_transactions_discarded() {
    let (mut client, mock_rpc_api, authenticator) = create_test_client().await;
    let (regular_account, faucet_account_header) =
        setup_wallet_and_faucet(&mut client, AccountStorageMode::Private, &authenticator).await;

    let account_id = regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    // Mint a note
    let note = mint_note(&mut client, account_id, faucet_account_id, NoteType::Private).await;
    consume_notes(&mut client, account_id, &[note]).await;

    // Create a transaction but don't submit it to the node
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();

    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], account_id, account_id),
        None,
        NoteType::Public,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    // Execute the transaction but don't submit it to the node
    let tx_result = client.new_transaction(account_id, tx_request).await.unwrap();
    let tx_id = tx_result.executed_transaction().id();
    client.testing_prove_transaction(&tx_result).await.unwrap();

    // Store the account state before applying the transaction
    let account_before_tx = client.get_account(account_id).await.unwrap().unwrap();
    let account_commitment_before_tx = account_before_tx.account().commitment();

    // Apply the transaction
    client.testing_apply_transaction(tx_result).await.unwrap();

    // Check that the account state has changed after applying the transaction
    let account_after_tx = client.get_account(account_id).await.unwrap().unwrap();
    let account_commitment_after_tx = account_after_tx.account().commitment();

    assert_ne!(
        account_commitment_before_tx, account_commitment_after_tx,
        "Account commitment should change after applying the transaction"
    );

    // Verify the transaction is in pending state
    let tx_record = client
        .get_transactions(TransactionFilter::All)
        .await
        .unwrap()
        .into_iter()
        .find(|tx| tx.id == tx_id)
        .unwrap();
    assert!(matches!(tx_record.transaction_status, TransactionStatus::Pending));

    // Sync the state, which should discard the old pending transaction
    mock_rpc_api.advance_blocks(TX_GRACEFUL_BLOCKS + 1);
    client.sync_state().await.unwrap();

    // Verify the transaction is now discarded
    let tx_record = client
        .get_transactions(TransactionFilter::All)
        .await
        .unwrap()
        .into_iter()
        .find(|tx| tx.id == tx_id)
        .unwrap();

    assert!(matches!(tx_record.transaction_status, TransactionStatus::Discarded));

    // Check that the account state has been rolled back after the transaction was discarded
    let account_after_sync = client.get_account(account_id).await.unwrap().unwrap();
    let account_commitment_after_sync = account_after_sync.account().commitment();

    assert_ne!(
        account_commitment_after_sync, account_commitment_after_tx,
        "Account commitment should change after transaction was discarded"
    );
    assert_eq!(
        account_commitment_after_sync, account_commitment_before_tx,
        "Account commitment should be rolled back to the value before the transaction"
    );
}
