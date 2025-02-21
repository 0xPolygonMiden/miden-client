use miden_client::{
    authenticator::keystore,
    note::NoteExecutionHint,
    store::NoteFilter,
    transaction::{InputNote, TransactionRequest, TransactionRequestBuilder},
    utils::{Deserializable, Serializable},
    ZERO,
};
use miden_objects::{
    account::{AccountId, AccountStorageMode},
    asset::FungibleAsset,
    crypto::{
        hash::rpo::Rpo256,
        merkle::{MerkleStore, MerkleTree, NodeIndex},
        rand::{FeltRng, RpoRandomCoin},
    },
    note::{
        Note, NoteAssets, NoteExecutionMode, NoteInputs, NoteMetadata, NoteRecipient, NoteTag,
        NoteType,
    },
    transaction::OutputNote,
    vm::AdviceMap,
    Felt, Word,
};

use super::common::*;

// CUSTOM TRANSACTION REQUEST
// ================================================================================================
//
// The following functions are for testing custom transaction code. What the test does is:
//
// - Create a custom tx that mints a custom note which checks that the note args are as expected
//   (ie, a word of 8 felts that represent [9, 12, 18, 3, 3, 18, 12, 9])
//      - The args will be provided via the advice map
//
// - Create another transaction that consumes this note with custom code. This custom code only
//   asserts that the {asserted_value} parameter is 0. To test this we first execute with an
//   incorrect value passed in, and after that we try again with the correct value.
//
// Because it's currently not possible to create/consume notes without assets, the P2ID code
// is used as the base for the note code.

const NOTE_ARGS: [Felt; 8] = [
    Felt::new(9),
    Felt::new(12),
    Felt::new(18),
    Felt::new(3),
    Felt::new(3),
    Felt::new(18),
    Felt::new(12),
    Felt::new(9),
];

#[tokio::test]
async fn test_transaction_request() {
    let (mut client, authenticator) = create_test_client().await;
    wait_for_node(&mut client).await;

    client.sync_state().await.unwrap();
    // Insert Account
    let (regular_account, _seed, _) =
        insert_new_wallet(&mut client, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap();

    let (fungible_faucet, _seed, _) =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap();

    // Execute mint transaction in order to create custom note
    let note = mint_custom_note(&mut client, fungible_faucet.id(), regular_account.id()).await;
    client.sync_state().await.unwrap();

    // Prepare transaction

    // If these args were to be modified, the transaction would fail because the note code expects
    // these exact arguments
    let note_args_commitment = Rpo256::hash_elements(&NOTE_ARGS);

    let note_args_map = vec![(note.id(), Some(note_args_commitment.into()))];
    let mut advice_map = AdviceMap::default();
    advice_map.insert(note_args_commitment, NOTE_ARGS.to_vec());

    let code = "
        use.miden::contracts::auth::basic->auth_tx

        begin
            push.0 push.{asserted_value}
            # => [0, {asserted_value}]
            assert_eq

            call.auth_tx::auth_tx_rpo_falcon512
        end
        ";
    // FAILURE ATTEMPT

    let failure_code = code.replace("{asserted_value}", "1");

    let tx_script = client.compile_tx_script(vec![], &failure_code).unwrap();

    let transaction_request = TransactionRequestBuilder::new()
        .with_authenticated_input_notes(note_args_map.clone())
        .with_custom_script(tx_script)
        .unwrap()
        .extend_advice_map(advice_map.clone())
        .build();

    // This fails becuase of {asserted_value} having the incorrect number passed in
    assert!(client.new_transaction(regular_account.id(), transaction_request).await.is_err());

    // SUCCESS EXECUTION

    let success_code = code.replace("{asserted_value}", "0");

    let tx_script = client.compile_tx_script(vec![], &success_code).unwrap();

    let transaction_request = TransactionRequestBuilder::new()
        .with_authenticated_input_notes(note_args_map)
        .with_custom_script(tx_script)
        .unwrap()
        .extend_advice_map(advice_map)
        .build();

    // TEST CUSTOM SCRIPT SERIALIZATION
    let mut buffer = Vec::new();
    transaction_request.write_into(&mut buffer);

    let deserialized_transaction_request = TransactionRequest::read_from_bytes(&buffer).unwrap();
    assert_eq!(transaction_request, deserialized_transaction_request);

    execute_tx_and_sync(&mut client, regular_account.id(), transaction_request).await;

    client.sync_state().await.unwrap();
}

#[tokio::test]
async fn test_merkle_store() {
    let (mut client, authenticator) = create_test_client().await;
    wait_for_node(&mut client).await;

    client.sync_state().await.unwrap();
    // Insert Account
    let (regular_account, _seed, _) =
        insert_new_wallet(&mut client, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap();

    let (fungible_faucet, _seed, _) =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap();

    // Execute mint transaction in order to increase nonce
    let note = mint_custom_note(&mut client, fungible_faucet.id(), regular_account.id()).await;
    client.sync_state().await.unwrap();

    // Prepare custom merkle store transaction

    // If these args were to be modified, the transaction would fail because the note code expects
    // these exact arguments
    let note_args_commitment = Rpo256::hash_elements(&NOTE_ARGS);

    let note_args_map = vec![(note.id(), Some(note_args_commitment.into()))];
    let mut advice_map = AdviceMap::new();
    advice_map.insert(note_args_commitment, NOTE_ARGS.to_vec());

    // Build merkle store and advice stack with merkle root
    let leaves: Vec<Word> =
        [1, 2, 3, 4].iter().map(|&v| [Felt::new(v), ZERO, ZERO, ZERO]).collect();
    let num_leaves = leaves.len();
    let merkle_tree = MerkleTree::new(leaves).unwrap();
    let merkle_root = merkle_tree.root();
    let merkle_store: MerkleStore = MerkleStore::from(&merkle_tree);

    let mut code = format!(
        "
                            use.std::collections::mmr
                            use.miden::contracts::auth::basic->auth_tx
                            use.miden::kernels::tx::prologue
                            use.miden::kernels::tx::memory

                            begin
                                # leaf count -> mem[4000][0]
                                push.{num_leaves} push.4000 mem_store

                                # merkle root -> mem[4004]
                                push.{} push.4004 mem_storew dropw
                        ",
        merkle_root.to_hex()
    );

    for pos in 0..(num_leaves as u64) {
        let expected_element = merkle_store
            .get_node(merkle_root, NodeIndex::new(2u8, pos).unwrap())
            .unwrap()
            .to_hex();
        code += format!(
            "
            # get element at index `pos` from the merkle store in mem[1000] and push it to stack
            push.4000 push.{pos} exec.mmr::get

            # check the element matches what was inserted at `pos`
            push.{expected_element} assert_eqw.err=999
        "
        )
        .as_str();
    }

    code += "call.auth_tx::auth_tx_rpo_falcon512 end";

    // Build the transaction
    let tx_script = client.compile_tx_script(vec![], &code).unwrap();

    let transaction_request = TransactionRequestBuilder::new()
        .with_authenticated_input_notes(note_args_map)
        .with_custom_script(tx_script)
        .unwrap()
        .extend_advice_map(advice_map)
        .extend_merkle_store(merkle_store.inner_nodes())
        .build();

    execute_tx_and_sync(&mut client, regular_account.id(), transaction_request).await;

    client.sync_state().await.unwrap();
}

#[tokio::test]
async fn test_onchain_notes_sync_with_tag() {
    // Client 1 has an private faucet which will mint an onchain note for client 2
    let (mut client_1, keystore_1) = create_test_client().await;
    // Client 2 will be used to sync and check that by adding the tag we can still fetch notes
    // whose tag doesn't necessarily match any of its accounts
    let (mut client_2, keystore_2) = create_test_client().await;
    // Client 3 will be the control client. We won't add any tags and expect the note not to be
    // fetched
    let (mut client_3, ..) = create_test_client().await;
    wait_for_node(&mut client_3).await;

    // Create accounts
    let (basic_account_1, ..) =
        insert_new_wallet(&mut client_1, AccountStorageMode::Private, &keystore_1)
            .await
            .unwrap();

    insert_new_wallet(&mut client_2, AccountStorageMode::Private, &keystore_2)
        .await
        .unwrap();

    client_1.sync_state().await.unwrap();
    client_2.sync_state().await.unwrap();
    client_3.sync_state().await.unwrap();

    // Create the custom note
    let note_script = "
            begin
                push.1 push.1
                assert_eq
            end
            ";
    let note_script = client_1.compile_note_script(note_script).unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let serial_num = client_1.rng().draw_word();
    let note_metadata = NoteMetadata::new(
        basic_account_1.id(),
        NoteType::Public,
        NoteTag::from_account_id(basic_account_1.id(), NoteExecutionMode::Local).unwrap(),
        NoteExecutionHint::None,
        Default::default(),
    )
    .unwrap();
    let note_assets = NoteAssets::new(vec![]).unwrap();
    let note_recipient = NoteRecipient::new(serial_num, note_script, inputs);
    let note = Note::new(note_assets, note_metadata, note_recipient);

    // Send transaction and wait for it to be committed
    let tx_request = TransactionRequestBuilder::new()
        .with_own_output_notes(vec![OutputNote::Full(note.clone())])
        .unwrap()
        .build();

    let note = tx_request.expected_output_notes().next().unwrap().clone();
    execute_tx_and_sync(&mut client_1, basic_account_1.id(), tx_request).await;

    // Load tag into client 2
    client_2
        .add_note_tag(
            NoteTag::from_account_id(basic_account_1.id(), NoteExecutionMode::Local).unwrap(),
        )
        .await
        .unwrap();

    // Client 2's account should receive the note here:
    client_2.sync_state().await.unwrap();
    client_3.sync_state().await.unwrap();

    // Assert that the note is the same
    let received_note: InputNote =
        client_2.get_input_note(note.id()).await.unwrap().unwrap().try_into().unwrap();
    assert_eq!(received_note.note().hash(), note.hash());
    assert_eq!(received_note.note(), &note);
    assert!(client_3.get_input_notes(NoteFilter::All).await.unwrap().is_empty());
}

async fn mint_custom_note(
    client: &mut TestClient,
    faucet_account_id: AccountId,
    target_account_id: AccountId,
) -> Note {
    // Prepare transaction
    let mut random_coin = RpoRandomCoin::new(Default::default());
    let note = create_custom_note(client, faucet_account_id, target_account_id, &mut random_coin);

    let transaction_request = TransactionRequestBuilder::new()
        .with_own_output_notes(vec![OutputNote::Full(note.clone())])
        .unwrap()
        .build();

    execute_tx_and_sync(client, faucet_account_id, transaction_request).await;
    note
}

fn create_custom_note(
    client: &TestClient,
    faucet_account_id: AccountId,
    target_account_id: AccountId,
    rng: &mut RpoRandomCoin,
) -> Note {
    let expected_note_args = NOTE_ARGS.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>();

    let mem_addr: u32 = 1000;

    let note_script = include_str!("asm/custom_p2id.masm")
        .replace("{expected_note_arg_1}", &expected_note_args[0..=3].join("."))
        .replace("{expected_note_arg_2}", &expected_note_args[4..=7].join("."))
        .replace("{mem_address}", &mem_addr.to_string())
        .replace("{mem_address_2}", &(mem_addr + 4).to_string());
    let note_script = client.compile_note_script(&note_script).unwrap();

    let inputs =
        NoteInputs::new(vec![target_account_id.prefix().as_felt(), target_account_id.suffix()])
            .unwrap();
    let serial_num = rng.draw_word();
    let note_metadata = NoteMetadata::new(
        faucet_account_id,
        NoteType::Private,
        NoteTag::from_account_id(target_account_id, NoteExecutionMode::Local).unwrap(),
        NoteExecutionHint::None,
        Default::default(),
    )
    .unwrap();
    let note_assets =
        NoteAssets::new(vec![FungibleAsset::new(faucet_account_id, 10).unwrap().into()]).unwrap();
    let note_recipient = NoteRecipient::new(serial_num, note_script, inputs);
    Note::new(note_assets, note_metadata, note_recipient)
}
