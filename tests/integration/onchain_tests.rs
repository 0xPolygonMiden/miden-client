use miden_client::{
    account::build_wallet_id,
    auth::AuthSecretKey,
    store::{InputNoteState, NoteFilter},
    transaction::{PaymentTransactionData, TransactionRequestBuilder},
};
use miden_objects::{
    account::{AccountId, AccountStorageMode},
    asset::{Asset, FungibleAsset},
    note::{NoteFile, NoteTag, NoteType},
    transaction::InputNote,
};
use rand::RngCore;

use super::common::*;

#[tokio::test]
async fn test_onchain_notes_flow() {
    // Client 1 is an private faucet which will mint an onchain note for client 2
    let (mut client_1, keystore_1) = create_test_client().await;
    // Client 2 is an private account which will consume the note that it will sync from the node
    let (mut client_2, keystore_2) = create_test_client().await;
    // Client 3 will be transferred part of the assets by client 2's account
    let (mut client_3, keystore_3) = create_test_client().await;
    wait_for_node(&mut client_3).await;

    // Create faucet account
    let (faucet_account, ..) =
        insert_new_fungible_faucet(&mut client_1, AccountStorageMode::Private, &keystore_1)
            .await
            .unwrap();

    // Create regular accounts
    let (basic_wallet_1, ..) =
        insert_new_wallet(&mut client_2, AccountStorageMode::Private, &keystore_2)
            .await
            .unwrap();

    // Create regular accounts
    let (basic_wallet_2, ..) =
        insert_new_wallet(&mut client_3, AccountStorageMode::Private, &keystore_3)
            .await
            .unwrap();

    client_1.sync_state().await.unwrap();
    client_2.sync_state().await.unwrap();

    let tx_request = TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
            basic_wallet_1.id(),
            NoteType::Public,
            client_1.rng(),
        )
        .unwrap();
    let note = tx_request.expected_output_notes().next().unwrap().clone();
    execute_tx_and_sync(&mut client_1, faucet_account.id(), tx_request).await;

    // Client 2's account should receive the note here:
    client_2.sync_state().await.unwrap();

    // Assert that the note is the same
    let received_note: InputNote =
        client_2.get_input_note(note.id()).await.unwrap().unwrap().try_into().unwrap();
    assert_eq!(received_note.note().commitment(), note.commitment());
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
    let tx_request = TransactionRequestBuilder::new()
        .build_pay_to_id(
            PaymentTransactionData::new(
                vec![p2id_asset.into()],
                basic_wallet_1.id(),
                basic_wallet_2.id(),
            ),
            None,
            NoteType::Public,
            client_2.rng(),
        )
        .unwrap();
    execute_tx_and_sync(&mut client_2, basic_wallet_1.id(), tx_request).await;

    // sync client 3 (basic account 2)
    client_3.sync_state().await.unwrap();
    // client 3 should only have one note
    let note = client_3
        .get_input_notes(NoteFilter::Committed)
        .await
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
    let (mut client_1, keystore_1) = create_test_client().await;
    let (mut client_2, keystore_2) = create_test_client().await;
    wait_for_node(&mut client_2).await;

    let (faucet_account_header, _, secret_key) =
        insert_new_fungible_faucet(&mut client_1, AccountStorageMode::Public, &keystore_1)
            .await
            .unwrap();

    let (first_regular_account, ..) =
        insert_new_wallet(&mut client_1, AccountStorageMode::Private, &keystore_1)
            .await
            .unwrap();

    let (second_client_first_regular_account, ..) =
        insert_new_wallet(&mut client_2, AccountStorageMode::Private, &keystore_2)
            .await
            .unwrap();

    let target_account_id = first_regular_account.id();
    let second_client_target_account_id = second_client_first_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    let (_, status) = client_1.get_account_header_by_id(faucet_account_id).await.unwrap().unwrap();
    let faucet_seed = status.seed().cloned();

    keystore_2.add_key(&AuthSecretKey::RpoFalcon512(secret_key)).unwrap();
    client_2.add_account(&faucet_account_header, faucet_seed, false).await.unwrap();

    // First Mint necesary token
    println!("First client consuming note");
    client_1.sync_state().await.unwrap();
    let note =
        mint_note(&mut client_1, target_account_id, faucet_account_id, NoteType::Private).await;

    // Update the state in the other client and ensure the onchain faucet commitment is consistent
    // between clients
    client_2.sync_state().await.unwrap();

    let (client_1_faucet, _) = client_1
        .get_account_header_by_id(faucet_account_header.id())
        .await
        .unwrap()
        .unwrap();
    let (client_2_faucet, _) = client_2
        .get_account_header_by_id(faucet_account_header.id())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(client_1_faucet.commitment(), client_2_faucet.commitment());

    // Now use the faucet in the second client to mint to its own account
    println!("Second client consuming note");
    let second_client_note = mint_note(
        &mut client_2,
        second_client_target_account_id,
        faucet_account_id,
        NoteType::Private,
    )
    .await;

    // Update the state in the other client and ensure the onchain faucet commitment is consistent
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

    let (client_1_faucet, _) = client_1
        .get_account_header_by_id(faucet_account_header.id())
        .await
        .unwrap()
        .unwrap();
    let (client_2_faucet, _) = client_2
        .get_account_header_by_id(faucet_account_header.id())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(client_1_faucet.commitment(), client_2_faucet.commitment());

    // Now we'll try to do a p2id transfer from an account of one client to the other one
    let from_account_id = target_account_id;
    let to_account_id = second_client_target_account_id;

    // get initial balances
    let from_account_balance = client_1
        .get_account(from_account_id)
        .await
        .unwrap()
        .unwrap()
        .account()
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let to_account_balance = client_2
        .get_account(to_account_id)
        .await
        .unwrap()
        .unwrap()
        .account()
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);

    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();

    println!("Running P2ID tx...");
    let tx_request = TransactionRequestBuilder::new()
        .build_pay_to_id(
            PaymentTransactionData::new(
                vec![Asset::Fungible(asset)],
                from_account_id,
                to_account_id,
            ),
            None,
            NoteType::Public,
            client_1.rng(),
        )
        .unwrap();
    execute_tx_and_sync(&mut client_1, from_account_id, tx_request).await;

    // sync on second client until we receive the note
    println!("Syncing on second client...");
    client_2.sync_state().await.unwrap();
    let notes = client_2.get_input_notes(NoteFilter::Committed).await.unwrap();

    //Import the note on the first client so that we can later check its consumer account
    client_1.import_note(NoteFile::NoteId(notes[0].id())).await.unwrap();

    // Consume the note
    println!("Consuming note on second client...");
    let tx_request = TransactionRequestBuilder::new()
        .build_consume_notes(vec![notes[0].id()])
        .unwrap();
    execute_tx_and_sync(&mut client_2, to_account_id, tx_request).await;

    // sync on first client
    println!("Syncing on first client...");
    client_1.sync_state().await.unwrap();

    // Check that the client doesn't know who consumed the note
    let input_note = client_1.get_input_note(notes[0].id()).await.unwrap().unwrap();
    assert!(matches!(input_note.state(), InputNoteState::ConsumedExternal { .. }));

    let new_from_account_balance = client_1
        .get_account(from_account_id)
        .await
        .unwrap()
        .unwrap()
        .account()
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let new_to_account_balance = client_2
        .get_account(to_account_id)
        .await
        .unwrap()
        .unwrap()
        .account()
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);

    assert_eq!(new_from_account_balance, from_account_balance - TRANSFER_AMOUNT);
    assert_eq!(new_to_account_balance, to_account_balance + TRANSFER_AMOUNT);
}

#[tokio::test]
async fn test_onchain_notes_sync_with_tag() {
    // Client 1 has an private faucet which will mint an onchain note for client 2
    let (mut client_1, keystore) = create_test_client().await;
    // Client 2 will be used to sync and check that by adding the tag we can still fetch notes
    // whose tag doesn't necessarily match any of its accounts
    let (mut client_2, _) = create_test_client().await;
    // Client 3 will be the control client. We won't add any tags and expect the note not to be
    // fetched
    let (mut client_3, _) = create_test_client().await;
    wait_for_node(&mut client_3).await;

    // Create faucet account
    let (faucet_account, ..) =
        insert_new_fungible_faucet(&mut client_1, AccountStorageMode::Private, &keystore)
            .await
            .unwrap();

    client_1.sync_state().await.unwrap();
    client_2.sync_state().await.unwrap();
    client_3.sync_state().await.unwrap();

    let target_account_id = AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap();

    let tx_request = TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
            target_account_id,
            NoteType::Public,
            client_1.rng(),
        )
        .unwrap();
    let note = tx_request.expected_output_notes().next().unwrap().clone();
    execute_tx_and_sync(&mut client_1, faucet_account.id(), tx_request).await;

    // Load tag into client 2
    client_2
        .add_note_tag(
            NoteTag::from_account_id(
                target_account_id,
                miden_objects::note::NoteExecutionMode::Local,
            )
            .unwrap(),
        )
        .await
        .unwrap();

    // Client 2's account should receive the note here:
    client_2.sync_state().await.unwrap();
    client_3.sync_state().await.unwrap();

    // Assert that the note is the same
    let received_note: InputNote =
        client_2.get_input_note(note.id()).await.unwrap().unwrap().try_into().unwrap();
    assert_eq!(received_note.note().commitment(), note.commitment());
    assert_eq!(received_note.note(), &note);
    assert!(client_3.get_input_notes(NoteFilter::All).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_import_account_by_id() {
    let (mut client_1, keystore_1) = create_test_client().await;
    let (mut client_2, keystore_2) = create_test_client().await;
    wait_for_node(&mut client_1).await;

    let mut user_seed = [0u8; 32];
    client_1.rng().fill_bytes(&mut user_seed);

    let (faucet_account_header, ..) =
        insert_new_fungible_faucet(&mut client_1, AccountStorageMode::Public, &keystore_1)
            .await
            .unwrap();

    let (first_regular_account, _, secret_key) = insert_new_wallet_with_seed(
        &mut client_1,
        AccountStorageMode::Public,
        &keystore_1,
        user_seed,
    )
    .await
    .unwrap();

    let target_account_id = first_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    // First mint and consume in the first client
    mint_and_consume(&mut client_1, target_account_id, faucet_account_id, NoteType::Public).await;

    // Mint a note for the second client
    let note =
        mint_note(&mut client_1, target_account_id, faucet_account_id, NoteType::Public).await;

    // Import the public account by id
    let anchor_block = client_1.get_latest_epoch_block().await.unwrap();
    let built_wallet_id = build_wallet_id(
        user_seed,
        secret_key.public_key(),
        AccountStorageMode::Public,
        false,
        &anchor_block,
    )
    .unwrap();
    assert_eq!(built_wallet_id, first_regular_account.id());
    client_2.import_account_by_id(built_wallet_id).await.unwrap();
    keystore_2.add_key(&AuthSecretKey::RpoFalcon512(secret_key)).unwrap();

    let original_account = client_1.get_account(first_regular_account.id()).await.unwrap().unwrap();
    let imported_account = client_2.get_account(first_regular_account.id()).await.unwrap().unwrap();
    assert_eq!(imported_account.account().commitment(), original_account.account().commitment());

    // Now use the wallet in the second client to consume the generated note
    println!("Second client consuming note");
    client_2.sync_state().await.unwrap();
    consume_notes(&mut client_2, target_account_id, &[note]).await;
    assert_account_has_single_asset(
        &client_2,
        target_account_id,
        faucet_account_id,
        MINT_AMOUNT * 2,
    )
    .await;
}
