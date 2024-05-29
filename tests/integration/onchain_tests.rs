use miden_client::{
    client::{
        accounts::AccountTemplate,
        transactions::transaction_request::{PaymentTransactionData, TransactionTemplate},
    },
    store::{NoteFilter, NoteStatus},
};
use miden_objects::{
    accounts::{AccountId, AccountStorageType},
    assets::{Asset, FungibleAsset, TokenSymbol},
    notes::{NoteTag, NoteType},
    transaction::InputNote,
};

use super::common::*;

#[tokio::test]
async fn test_onchain_notes_flow() {
    // Client 1 is an offchain faucet which will mint an onchain note for client 2
    let mut client_1 = create_test_client();
    // Client 2 is an offchain account which will consume the note that it will sync from the node
    let mut client_2 = create_test_client();
    // Client 3 will be transferred part of the assets by client 2's account
    let mut client_3 = create_test_client();
    wait_for_node(&mut client_3).await;

    // Create faucet account
    let (faucet_account, _) = client_1
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("MATIC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000_000,
            storage_type: AccountStorageType::OffChain,
        })
        .unwrap();

    // Create regular accounts
    let (basic_wallet_1, _) = client_2
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_type: AccountStorageType::OffChain,
        })
        .unwrap();

    // Create regular accounts
    let (basic_wallet_2, _) = client_3
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_type: AccountStorageType::OffChain,
        })
        .unwrap();
    client_1.sync_state().await.unwrap();
    client_2.sync_state().await.unwrap();

    let tx_template = TransactionTemplate::MintFungibleAsset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        basic_wallet_1.id(),
        NoteType::Public,
    );

    let tx_request = client_1.build_transaction_request(tx_template).unwrap();
    let note = tx_request.expected_output_notes()[0].clone();
    execute_tx_and_sync(&mut client_1, tx_request).await;

    // Client 2's account should receive the note here:
    client_2.sync_state().await.unwrap();

    // Assert that the note is the same
    let received_note: InputNote = client_2.get_input_note(note.id()).unwrap().try_into().unwrap();
    assert_eq!(received_note.note().authentication_hash(), note.authentication_hash());
    assert_eq!(received_note.note(), &note);

    // consume the note
    consume_notes(&mut client_2, basic_wallet_1.id(), &[received_note]).await;
    assert_account_has_single_asset(
        &client_2,
        basic_wallet_1.id(),
        faucet_account.id(),
        MINT_AMOUNT,
    )
    .await;

    let p2id_asset = FungibleAsset::new(faucet_account.id(), TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(
        PaymentTransactionData::new(p2id_asset.into(), basic_wallet_1.id(), basic_wallet_2.id()),
        NoteType::Public,
    );
    let tx_request = client_2.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client_2, tx_request).await;

    // sync client 3 (basic account 2)
    client_3.sync_state().await.unwrap();
    // client 3 should only have one note
    let note = client_3
        .get_input_notes(NoteFilter::Committed)
        .unwrap()
        .first()
        .unwrap()
        .clone()
        .try_into()
        .unwrap();

    consume_notes(&mut client_3, basic_wallet_2.id(), &[note]).await;
    assert_account_has_single_asset(
        &client_3,
        basic_wallet_2.id(),
        faucet_account.id(),
        TRANSFER_AMOUNT,
    )
    .await;
}

#[tokio::test]
async fn test_onchain_accounts() {
    let mut client_1 = create_test_client();
    let mut client_2 = create_test_client();
    wait_for_node(&mut client_2).await;

    let (first_regular_account, _second_regular_account, faucet_account_stub) =
        setup(&mut client_1, AccountStorageType::OnChain).await;

    let (
        second_client_first_regular_account,
        _other_second_regular_account,
        _other_faucet_account_stub,
    ) = setup(&mut client_2, AccountStorageType::OffChain).await;

    let target_account_id = first_regular_account.id();
    let second_client_target_account_id = second_client_first_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    let (_, faucet_seed) = client_1.get_account_stub_by_id(faucet_account_id).unwrap();
    let auth_info = client_1.get_account_auth(faucet_account_id).unwrap();
    client_2.insert_account(&faucet_account_stub, faucet_seed, &auth_info).unwrap();

    // First Mint necesary token
    println!("First client consuming note");
    let note =
        mint_note(&mut client_1, target_account_id, faucet_account_id, NoteType::OffChain).await;

    // Update the state in the other client and ensure the onchain faucet hash is consistent
    // between clients
    client_2.sync_state().await.unwrap();

    let (client_1_faucet, _) = client_1.get_account_stub_by_id(faucet_account_stub.id()).unwrap();
    let (client_2_faucet, _) = client_2.get_account_stub_by_id(faucet_account_stub.id()).unwrap();

    assert_eq!(client_1_faucet.hash(), client_2_faucet.hash());

    // Now use the faucet in the second client to mint to its own account
    println!("Second client consuming note");
    let second_client_note = mint_note(
        &mut client_2,
        second_client_target_account_id,
        faucet_account_id,
        NoteType::OffChain,
    )
    .await;

    // Update the state in the other client and ensure the onchain faucet hash is consistent
    // between clients
    client_1.sync_state().await.unwrap();

    println!("About to consume");
    consume_notes(&mut client_1, target_account_id, &[note]).await;
    assert_account_has_single_asset(&client_1, target_account_id, faucet_account_id, MINT_AMOUNT)
        .await;
    consume_notes(&mut client_2, second_client_target_account_id, &[second_client_note]).await;
    assert_account_has_single_asset(
        &client_2,
        second_client_target_account_id,
        faucet_account_id,
        MINT_AMOUNT,
    )
    .await;

    let (client_1_faucet, _) = client_1.get_account_stub_by_id(faucet_account_stub.id()).unwrap();
    let (client_2_faucet, _) = client_2.get_account_stub_by_id(faucet_account_stub.id()).unwrap();

    assert_eq!(client_1_faucet.hash(), client_2_faucet.hash());

    // Now we'll try to do a p2id transfer from an account of one client to the other one
    let from_account_id = target_account_id;
    let to_account_id = second_client_target_account_id;

    // get initial balances
    let from_account_balance = client_1
        .get_account(from_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let to_account_balance = client_2
        .get_account(to_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);

    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        NoteType::Public,
    );

    println!("Running P2ID tx...");
    let tx_request = client_1.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client_1, tx_request).await;

    // sync on second client until we receive the note
    println!("Syncing on second client...");
    client_2.sync_state().await.unwrap();
    let notes = client_2.get_input_notes(NoteFilter::Committed).unwrap();

    //Import the note on the first client so that we can later check its consumer account
    client_1.import_input_note(notes[0].clone(), false).await.unwrap();

    // Consume the note
    println!("Consuming note con second client...");
    let tx_template = TransactionTemplate::ConsumeNotes(to_account_id, vec![notes[0].id()]);
    let tx_request = client_2.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client_2, tx_request).await;

    // sync on first client
    println!("Syncing on first client...");
    client_1.sync_state().await.unwrap();

    // Check that the client doesn't know who consumed the note
    let input_note = client_1.get_input_note(notes[0].id()).unwrap();
    assert!(matches!(input_note.status(), NoteStatus::Consumed));
    assert!(input_note.consumer_account_id().is_none());

    let new_from_account_balance = client_1
        .get_account(from_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let new_to_account_balance = client_2
        .get_account(to_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);

    assert_eq!(new_from_account_balance, from_account_balance - TRANSFER_AMOUNT);
    assert_eq!(new_to_account_balance, to_account_balance + TRANSFER_AMOUNT);
}

#[tokio::test]
async fn test_onchain_notes_sync_with_tag() {
    // Client 1 has an offchain faucet which will mint an onchain note for client 2
    let mut client_1 = create_test_client();
    // Client 2 will be used to sync and check that by adding the tag we can still fetch notes
    // whose tag doesn't necessarily match any of its accounts
    let mut client_2 = create_test_client();
    // Client 3 will be the control client. We won't add any tags and expect the note not to be
    // fetched
    let mut client_3 = create_test_client();
    wait_for_node(&mut client_3).await;

    // Create faucet account
    let (faucet_account, _) = client_1
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("MATIC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000_000,
            storage_type: AccountStorageType::OffChain,
        })
        .unwrap();

    client_1.sync_state().await.unwrap();
    client_2.sync_state().await.unwrap();
    client_3.sync_state().await.unwrap();

    let target_account_id = AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap();
    let tx_template = TransactionTemplate::MintFungibleAsset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        target_account_id,
        NoteType::Public,
    );

    let tx_request = client_1.build_transaction_request(tx_template).unwrap();
    let note = tx_request.expected_output_notes()[0].clone();
    execute_tx_and_sync(&mut client_1, tx_request).await;

    // Load tag into client 2
    client_2
        .add_note_tag(
            NoteTag::from_account_id(
                target_account_id,
                miden_objects::notes::NoteExecutionHint::Local,
            )
            .unwrap(),
        )
        .unwrap();

    // Client 2's account should receive the note here:
    client_2.sync_state().await.unwrap();
    client_3.sync_state().await.unwrap();

    // Assert that the note is the same
    let received_note: InputNote = client_2.get_input_note(note.id()).unwrap().try_into().unwrap();
    assert_eq!(received_note.note().authentication_hash(), note.authentication_hash());
    assert_eq!(received_note.note(), &note);
    assert!(client_3.get_input_notes(NoteFilter::All).unwrap().is_empty());
}
