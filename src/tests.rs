// TESTS
// ================================================================================================
use crate::{
    client::{
        accounts::{AccountStorageMode, AccountTemplate},
        Client,
    },
    config::{ClientConfig, Endpoint},
    store::{
        accounts::AuthInfo,
        notes::{InputNoteFilter, InputNoteRecord},
        tests::create_test_store_path,
    },
};

use crypto::{
    dsa::rpo_falcon512::KeyPair,
    merkle::{InOrderIndex, MerklePath, MmrPeaks, PartialMmr},
    Felt, FieldElement,
};
use miden_lib::transaction::TransactionKernel;
use mock::{
    constants::{generate_account_seed, AccountSeedType},
    mock::{
        account::{self, MockAccountType},
        notes::AssetPreservationStatus,
        transaction::mock_inputs,
    },
};
use objects::{
    accounts::{AccountId, AccountStub},
    assets::TokenSymbol,
    transaction::InputNotes,
    Digest,
};

#[tokio::test]
async fn test_input_notes_round_trip() {
    // generate test store path
    let store_path = create_test_store_path();

    // generate test client
    let mut client = Client::new(ClientConfig::new(
        store_path.into_os_string().into_string().unwrap(),
        Endpoint::default(),
    ))
    .await
    .unwrap();

    // generate test data
    let transaction_inputs = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );
    let recorded_notes = transaction_inputs.input_notes();

    // insert notes into database
    for note in recorded_notes.iter().cloned() {
        client.import_input_note(note.into()).unwrap();
    }

    // retrieve notes from database
    let retrieved_notes = client.get_input_notes(InputNoteFilter::Committed).unwrap();

    let recorded_notes: Vec<InputNoteRecord> =
        recorded_notes.iter().map(|n| n.clone().into()).collect();
    // compare notes
    for (recorded_note, retrieved_note) in recorded_notes.iter().zip(retrieved_notes) {
        assert_eq!(recorded_note.note_id(), retrieved_note.note_id());
    }
}

#[tokio::test]
async fn test_get_input_note() {
    // generate test store path
    let store_path = create_test_store_path();

    // generate test client
    let mut client = Client::new(ClientConfig::new(
        store_path.into_os_string().into_string().unwrap(),
        Endpoint::default(),
    ))
    .await
    .unwrap();

    // generate test data
    let transaction_inputs = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );
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
    // generate test store path
    let store_path = create_test_store_path();

    // generate test client
    let mut client = Client::new(ClientConfig::new(
        store_path.into_os_string().into_string().unwrap(),
        Endpoint::default(),
    ))
    .await
    .unwrap();

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
    assert_eq!(account.vault().commitment(), fetched_account.vault_root());
    assert_eq!(account.storage().root(), fetched_account.storage_root());
    assert_eq!(account.code().root(), fetched_account.code_root());

    // Validate seed matches
    assert_eq!(account_seed, fetched_account_seed);
}

#[tokio::test]
async fn insert_faucet_account() {
    // generate test store path
    let store_path = create_test_store_path();

    // generate test client
    let mut client = Client::new(ClientConfig::new(
        store_path.into_os_string().into_string().unwrap(),
        Endpoint::default(),
    ))
    .await
    .unwrap();

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
    assert_eq!(account.vault().commitment(), fetched_account.vault_root());
    assert_eq!(account.storage().root(), fetched_account.storage_root());
    assert_eq!(account.code().root(), fetched_account.code_root());

    // Validate seed matches
    assert_eq!(account_seed, fetched_account_seed);
}

#[tokio::test]
async fn insert_same_account_twice_fails() {
    // generate test store path
    let store_path = create_test_store_path();

    // generate test client
    let mut client = Client::new(ClientConfig::new(
        store_path.into_os_string().into_string().unwrap(),
        Endpoint::default(),
    ))
    .await
    .unwrap();

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
async fn test_get_account_by_id() {
    // generate test store path
    let store_path = create_test_store_path();

    // generate test client
    let mut client = Client::new(ClientConfig::new(
        store_path.into_os_string().into_string().unwrap(),
        Endpoint::default(),
    ))
    .await
    .unwrap();

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
    let (acc_from_db, _account_seed) = match client.get_account_by_id(account.id()) {
        Ok(account) => account,
        Err(err) => panic!("Error retrieving account: {}", err),
    };
    assert_eq!(AccountStub::from(account), acc_from_db);

    // Retrieving a non existing account should fail
    let hex = format!("0x{}", "1".repeat(16));
    let invalid_id = AccountId::from_hex(&hex).unwrap();
    assert!(client.get_account_by_id(invalid_id).is_err());
}

#[tokio::test]
async fn test_sync_state() {
    // generate test store path
    let store_path = create_test_store_path();

    // generate test client
    let mut client = Client::new(ClientConfig::new(
        store_path.into_os_string().into_string().unwrap(),
        Endpoint::default(),
    ))
    .await
    .unwrap();

    // generate test data
    let (last_block_header, _chain_mmr) = crate::mock::insert_mock_data(&mut client);

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
        client.get_latest_block_num().unwrap(),
        client
            .rpc_api
            .sync_state_requests
            .first_key_value()
            .unwrap()
            .1
            .chain_tip
    );

    // verify that we inserted the latest block into the db via the client
    let latest_block = client.get_latest_block_num().unwrap();
    assert_eq!(block_num, latest_block);
    assert_eq!(
        last_block_header,
        client
            .get_block_headers(latest_block, latest_block)
            .unwrap()[0]
    );

    // Try reconstructing the chain_mmr from what's in the database
    let partial_mmr = build_partial_mmr_from_client_state(&mut client);

    // Since Mocked data contains two sync updates we should be "tracking" those blocks
    assert!(partial_mmr.open(0).unwrap().is_none());
    assert!(partial_mmr.open(1).unwrap().is_none());
    assert!(partial_mmr.open(2).unwrap().is_some());
    assert!(partial_mmr.open(3).unwrap().is_none());
    assert!(partial_mmr.open(4).unwrap().is_some());
}

fn build_partial_mmr_from_client_state(client: &mut Client) -> PartialMmr {
    // let current_peaks_hash = client
    //             .get_chain_mmr_peaks_by_block_num(block_num)
    //             .unwrap();
    //
    // assert_eq!(
    //     current_peaks.hash(),
    //     chain_mmr.peaks().hash_peaks()
    // );
    //
    // let partial_mmr = PartialMmr::from_peaks(current_peaks)
    let mut partial_mmr = PartialMmr::from_peaks(MmrPeaks::new(0, Vec::new()).unwrap());
    let chain_mmr_authentication_nodes = client.get_chain_mmr_nodes().unwrap();

    let block_headers = client
        .get_block_headers(0, client.get_latest_block_num().unwrap())
        .unwrap();

    let tracked_nodes: Vec<(usize, Digest)> = block_headers
        .iter()
        .map(|block_header| (block_header.block_num() as usize, block_header.hash()))
        .collect();

    for (block_num, node_hash) in tracked_nodes {
        let mut nodes = Vec::new();
        let mut idx = InOrderIndex::from_leaf_pos(block_num);

        while let Some(node) = chain_mmr_authentication_nodes.get(&idx.sibling()) {
            nodes.push(*node);
            idx = idx.parent();
        }
        partial_mmr
            .add(block_num as usize, node_hash, &MerklePath::new(nodes))
            .unwrap();
    }

    partial_mmr
}

#[tokio::test]
async fn test_add_tag() {
    // generate test store path
    let store_path = create_test_store_path();

    // generate test client
    let mut client = Client::new(ClientConfig::new(
        store_path.into_os_string().into_string().unwrap(),
        Endpoint::default(),
    ))
    .await
    .unwrap();

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
