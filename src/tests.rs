// TESTS
// ================================================================================================
use crate::{
    client::Client,
    config::{ClientConfig, Endpoint},
    store::{accounts::AuthInfo, notes::InputNoteFilter, tests::create_test_store_path},
};

use crypto::dsa::rpo_falcon512::KeyPair;
use miden_lib::assembler::assembler;
use mock::mock::{
    account::{self, MockAccountType},
    notes::AssetPreservationStatus,
    transaction::mock_inputs,
};
use objects::{
    accounts::{AccountId, AccountStub},
    AdviceInputs,
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
    let (_, _, _, recorded_notes, _) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    // insert notes into database
    for note in recorded_notes.iter().cloned() {
        client.insert_input_note(note).unwrap();
    }

    // retrieve notes from database
    let retrieved_notes = client.get_input_notes(InputNoteFilter::All).unwrap();

    // compare notes
    assert_eq!(recorded_notes, retrieved_notes);
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
    let (_, _, _, recorded_notes, _) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    // insert note into database
    client.insert_input_note(recorded_notes[0].clone()).unwrap();

    // retrieve note from database
    let retrieved_note = client
        .get_input_note(recorded_notes[0].note().hash())
        .unwrap();

    // compare notes
    assert_eq!(recorded_notes[0], retrieved_note);
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

    let assembler = assembler();
    let mut auxiliary_data = AdviceInputs::default();
    let account = account::mock_new_account(&assembler, &mut auxiliary_data);

    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();

    assert!(client
        .insert_account(&account, &AuthInfo::RpoFalcon512(key_pair))
        .is_ok());
    assert!(client
        .insert_account(&account, &AuthInfo::RpoFalcon512(key_pair))
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

    let assembler = assembler();
    let mut auxiliary_data = AdviceInputs::default();
    let account = account::mock_new_account(&assembler, &mut auxiliary_data);

    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();

    client
        .insert_account(&account, &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();

    // Retrieving an existing account should succeed
    let acc_from_db = match client.get_account_by_id(account.id()) {
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
    crate::mock::insert_mock_data(&mut client);

    // assert that we have no consumed notes prior to syncing state
    assert_eq!(
        client
            .get_input_notes(InputNoteFilter::Consumed)
            .unwrap()
            .len(),
        0
    );

    // sync state
    let block_num = client.sync_state().await.unwrap();

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

    // verify that the latest block number has been updated
    assert_eq!(
        client.get_latest_block_number().unwrap(),
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
