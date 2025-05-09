use std::{sync::Arc, time::Duration};

use miden_client::{
    ClientError, ONE,
    account::Account,
    builder::ClientBuilder,
    note::NoteRelevance,
    rpc::{Endpoint, NodeRpcClient, TonicRpcClient, domain::account::AccountDetails},
    store::{
        InputNoteRecord, InputNoteState, NoteFilter, OutputNoteState, TransactionFilter,
        input_note_states::ConsumedAuthenticatedLocalNoteState,
    },
    sync::{NoteTagSource, TX_GRACEFUL_BLOCKS},
    transaction::{
        PaymentTransactionData, TransactionExecutorError, TransactionProver,
        TransactionProverError, TransactionRequestBuilder, TransactionStatus,
    },
};
use miden_objects::{
    account::{AccountId, AccountStorageMode},
    asset::{Asset, FungibleAsset},
    note::{NoteFile, NoteType},
    transaction::{ProvenTransaction, ToInputNoteCommitments, TransactionWitness},
};

mod common;
use common::*;
use winter_maybe_async::maybe_async_trait;

mod custom_transactions_tests;
mod fpi_tests;
mod onchain_tests;
mod swap_transactions_tests;

/// Constant that represents the number of blocks until the p2idr can be recalled. If this value is
/// too low, some tests might fail due to expected recall failures not happening.
const RECALL_HEIGHT_DELTA: u32 = 50;

#[tokio::test]
async fn test_client_builder_initializes_client_with_endpoint() -> Result<(), ClientError> {
    let (_, _, store_config, auth_path) = get_client_config();

    let mut client = ClientBuilder::new()
        .with_tonic_rpc_client(&Endpoint::default(), Some(10_000))
        .with_filesystem_keystore(auth_path.to_str().unwrap())
        .with_sqlite_store(store_config.to_str().unwrap())
        .in_debug_mode(true)
        .build()
        .await?;

    assert!(client.is_in_debug_mode());

    let sync_summary = client.sync_state().await.expect("Sync state failed");

    assert!(sync_summary.block_num.as_u32() > 0);

    Ok(())
}

#[tokio::test]
async fn test_client_builder_initializes_client_with_rpc() -> Result<(), ClientError> {
    let (_, _, store_config, auth_path) = get_client_config();

    let endpoint =
        Endpoint::new("https".to_string(), "rpc.testnet.miden.io".to_string(), Some(443));
    let timeout_ms = 10_000;
    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    let mut client = ClientBuilder::new()
        .with_rpc(rpc_api)
        .with_filesystem_keystore(auth_path.to_str().unwrap())
        .with_sqlite_store(store_config.to_str().unwrap())
        .in_debug_mode(true)
        .build()
        .await?;

    assert!(client.is_in_debug_mode());

    let sync_summary = client.sync_state().await.expect("Sync state failed");

    assert!(sync_summary.block_num.as_u32() > 0);

    Ok(())
}

#[tokio::test]
async fn test_client_builder_fails_without_keystore() {
    let (_, _, store_config, _) = get_client_config();
    let result = ClientBuilder::new()
        .with_tonic_rpc_client(&Endpoint::default(), Some(10_000))
        .with_sqlite_store(store_config.to_str().unwrap())
        .in_debug_mode(true)
        .build()
        .await;

    assert!(result.is_err(), "Expected client build to fail without a keystore");
}

#[tokio::test]
async fn test_added_notes() {
    let (mut client, authenticator) = create_test_client().await;
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
    assert!(notes.is_empty())
}

#[tokio::test]
async fn test_multiple_tx_on_same_block() {
    let (mut client, authenticator) = create_test_client().await;
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
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_request_1 = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        None,
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    let tx_request_2 = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        None,
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    println!("Running P2ID tx...");

    // Create transactions
    let transaction_execution_result_1 =
        client.new_transaction(from_account_id, tx_request_1).await.unwrap();
    let transaction_id_1 = transaction_execution_result_1.executed_transaction().id();
    let tx_prove_1 =
        client.testing_prove_transaction(&transaction_execution_result_1).await.unwrap();
    client.testing_apply_transaction(transaction_execution_result_1).await.unwrap();

    let transaction_execution_result_2 =
        client.new_transaction(from_account_id, tx_request_2).await.unwrap();
    let transaction_id_2 = transaction_execution_result_2.executed_transaction().id();
    let tx_prove_2 =
        client.testing_prove_transaction(&transaction_execution_result_2).await.unwrap();
    client.testing_apply_transaction(transaction_execution_result_2).await.unwrap();

    client.sync_state().await.unwrap();

    // wait for 1 block
    wait_for_blocks(&mut client, 1).await;

    // Submit the proven transactions
    client.testing_submit_proven_transaction(tx_prove_1).await.unwrap();
    client.testing_submit_proven_transaction(tx_prove_2).await.unwrap();

    // wait for 1 block
    wait_for_tx(&mut client, transaction_id_1).await;

    let transactions = client
        .get_transactions(crate::TransactionFilter::All)
        .await
        .unwrap()
        .into_iter()
        .filter(|tx| tx.id == transaction_id_1 || tx.id == transaction_id_2)
        .collect::<Vec<_>>();

    assert_eq!(transactions.len(), 2);
    assert!(matches!(
        transactions[0].transaction_status,
        TransactionStatus::Committed { .. }
    ));
    assert_eq!(transactions[0].transaction_status, transactions[1].transaction_status);

    let note_id = transactions[0].output_notes.iter().next().unwrap().id();
    let note = client.get_output_note(note_id).await.unwrap().unwrap();
    assert!(matches!(note.state(), OutputNoteState::CommittedFull { .. }));

    let sender_account = client.get_account(from_account_id).await.unwrap().unwrap();
    assert_eq!(
        sender_account.account().vault().get_balance(faucet_account_id).unwrap(),
        MINT_AMOUNT - (TRANSFER_AMOUNT * 2)
    );
}

#[tokio::test]
async fn test_p2id_transfer() {
    let (mut client, authenticator) = create_test_client().await;
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
    let seed = regular_account.seed().cloned();
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
    let (mut client, authenticator) = create_test_client().await;
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
    let (mut client, authenticator) = create_test_client().await;
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
    let (mut client, authenticator) = create_test_client().await;
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
    wait_for_blocks(&mut client, RECALL_HEIGHT_DELTA).await;

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

// TODO: Change this to a unit test
#[tokio::test]
async fn test_get_consumable_notes() {
    let (mut client, authenticator) = create_test_client().await;

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
    let current_block_num = client.get_sync_height().await.unwrap();
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

    // Check that note is consumable by both accounts
    let consumable_notes = client.get_consumable_notes(None).await.unwrap();
    let relevant_accounts = &consumable_notes.first().unwrap().1;
    assert_eq!(relevant_accounts.len(), 2);
    assert!(!client.get_consumable_notes(Some(from_account_id)).await.unwrap().is_empty());
    assert!(!client.get_consumable_notes(Some(to_account_id)).await.unwrap().is_empty());

    // Check that the note is only consumable after the specified block for the account that sent
    // the transaction
    let from_account_relevance = relevant_accounts
        .iter()
        .find(|relevance| relevance.0 == from_account_id)
        .unwrap()
        .1;
    assert_eq!(
        from_account_relevance,
        NoteRelevance::After(current_block_num.as_u32() + RECALL_HEIGHT_DELTA)
    );

    // Check that the note is always consumable for the account that received the transaction
    let to_account_relevance = relevant_accounts
        .iter()
        .find(|relevance| relevance.0 == to_account_id)
        .unwrap()
        .1;
    assert_eq!(to_account_relevance, NoteRelevance::Now);

    wait_for_blocks(&mut client, RECALL_HEIGHT_DELTA).await;

    // After waiting, the note is consumable by the sender account
    let consumable_notes = client.get_consumable_notes(None).await.unwrap();
    let relevant_accounts = &consumable_notes.first().unwrap().1;
    let from_account_relevance = relevant_accounts
        .iter()
        .find(|relevance| relevance.0 == from_account_id)
        .unwrap()
        .1;
    assert_eq!(from_account_relevance, NoteRelevance::Now);
}

#[tokio::test]
async fn test_get_output_notes() {
    let (mut client, authenticator) = create_test_client().await;

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
async fn test_import_expected_notes() {
    let (mut client_1, authenticator_1) = create_test_client().await;
    let (first_basic_account, faucet_account) =
        setup_wallet_and_faucet(&mut client_1, AccountStorageMode::Private, &authenticator_1).await;

    let (mut client_2, authenticator_2) = create_test_client().await;
    let (client_2_account, _seed, _) =
        insert_new_wallet(&mut client_2, AccountStorageMode::Private, &authenticator_2)
            .await
            .unwrap();

    wait_for_node(&mut client_2).await;

    let tx_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        client_2_account.id(),
        NoteType::Public,
        client_2.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    let note: InputNoteRecord = tx_request.expected_output_notes().next().unwrap().clone().into();
    client_2.sync_state().await.unwrap();

    // If the verification is requested before execution then the import should fail
    assert!(client_2.import_note(NoteFile::NoteId(note.id())).await.is_err());
    execute_tx_and_sync(&mut client_1, faucet_account.id(), tx_request).await;

    // Use client 1 to wait until a couple of blocks have passed
    wait_for_blocks(&mut client_1, 3).await;

    let new_sync_data = client_2.sync_state().await.unwrap();

    client_2.add_note_tag(note.metadata().unwrap().tag()).await.unwrap();
    client_2.import_note(NoteFile::NoteId(note.clone().id())).await.unwrap();
    client_2.sync_state().await.unwrap();
    let input_note = client_2.get_input_note(note.id()).await.unwrap().unwrap();
    assert!(
        new_sync_data.block_num > input_note.inclusion_proof().unwrap().location().block_num() + 1
    );

    // If imported after execution and syncing then the inclusion proof should be Some
    assert!(input_note.inclusion_proof().is_some());

    // If client 2 succesfully consumes the note, we confirm we have MMR and block header data
    consume_notes(&mut client_2, client_2_account.id(), &[input_note.try_into().unwrap()]).await;

    let tx_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        first_basic_account.id(),
        NoteType::Private,
        client_2.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    let note: InputNoteRecord = tx_request.expected_output_notes().next().unwrap().clone().into();

    // Import an uncommited note without verification
    client_2.add_note_tag(note.metadata().unwrap().tag()).await.unwrap();
    client_2
        .import_note(NoteFile::NoteDetails {
            details: note.clone().into(),
            after_block_num: client_1.get_sync_height().await.unwrap(),
            tag: Some(note.metadata().unwrap().tag()),
        })
        .await
        .unwrap();
    let input_note = client_2.get_input_note(note.id()).await.unwrap().unwrap();

    // If imported before execution then the inclusion proof should be None
    assert!(input_note.inclusion_proof().is_none());

    execute_tx_and_sync(&mut client_1, faucet_account.id(), tx_request).await;
    client_2.sync_state().await.unwrap();

    // After sync, the imported note should have inclusion proof even if it's not relevant for its
    // accounts.
    let input_note = client_2.get_input_note(note.id()).await.unwrap().unwrap();
    assert!(input_note.inclusion_proof().is_some());

    // If inclusion proof is invalid this should panic
    consume_notes(&mut client_1, first_basic_account.id(), &[input_note.try_into().unwrap()]).await;
}

#[tokio::test]
async fn test_import_expected_note_uncommitted() {
    let (mut client_1, authenticator) = create_test_client().await;
    let faucet_account =
        insert_new_fungible_faucet(&mut client_1, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap()
            .0;

    let (mut client_2, _) = create_test_client().await;
    let (client_2_account, _seed, _) =
        insert_new_wallet(&mut client_2, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap();

    wait_for_node(&mut client_2).await;

    let tx_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        client_2_account.id(),
        NoteType::Public,
        client_1.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    let note: InputNoteRecord = tx_request.expected_output_notes().next().unwrap().clone().into();
    client_2.sync_state().await.unwrap();

    // If the verification is requested before execution then the import should fail
    let imported_note_id = client_2
        .import_note(NoteFile::NoteDetails {
            details: note.into(),
            after_block_num: 0.into(),
            tag: None,
        })
        .await
        .unwrap();

    let imported_note = client_2.get_input_note(imported_note_id).await.unwrap().unwrap();

    assert!(matches!(imported_note.state(), InputNoteState::Expected { .. }));
}

#[tokio::test]
async fn test_import_expected_notes_from_the_past_as_committed() {
    let (mut client_1, authenticator_1) = create_test_client().await;
    let (first_basic_account, faucet_account) =
        setup_wallet_and_faucet(&mut client_1, AccountStorageMode::Private, &authenticator_1).await;

    let (mut client_2, _) = create_test_client().await;

    wait_for_node(&mut client_2).await;

    let tx_request = TransactionRequestBuilder::mint_fungible_asset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        first_basic_account.id(),
        NoteType::Public,
        client_1.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    let note: InputNoteRecord = tx_request.expected_output_notes().next().unwrap().clone().into();

    let block_height_before = client_1.get_sync_height().await.unwrap();

    execute_tx_and_sync(&mut client_1, faucet_account.id(), tx_request).await;

    // Use client 1 to wait until a couple of blocks have passed
    wait_for_blocks(&mut client_1, 3).await;
    client_2.sync_state().await.unwrap();

    // If the verification is requested before execution then the import should fail
    let note_id = client_2
        .import_note(NoteFile::NoteDetails {
            details: note.clone().into(),
            after_block_num: block_height_before,
            tag: Some(note.metadata().unwrap().tag()),
        })
        .await
        .unwrap();

    let imported_note = client_2.get_input_note(note_id).await.unwrap().unwrap();

    // Get the note status in client 1
    let client_1_note = client_1.get_input_note(note_id).await.unwrap().unwrap();

    assert_eq!(imported_note.state(), client_1_note.state());
}

#[tokio::test]
async fn test_get_account_update() {
    // Create a client with both public and private accounts.
    let (mut client, authenticator) = create_test_client().await;

    let (basic_wallet_1, faucet_account) =
        setup_wallet_and_faucet(&mut client, AccountStorageMode::Private, &authenticator).await;
    wait_for_node(&mut client).await;

    let (basic_wallet_2, ..) =
        insert_new_wallet(&mut client, AccountStorageMode::Public, &authenticator)
            .await
            .unwrap();

    // Mint and consume notes with both accounts so they are included in the node.
    mint_and_consume(&mut client, basic_wallet_1.id(), faucet_account.id(), NoteType::Private)
        .await;
    mint_and_consume(&mut client, basic_wallet_2.id(), faucet_account.id(), NoteType::Private)
        .await;

    // Request updates from node for both accounts. The request should not fail and both types of
    // [`AccountDetails`] should be received.
    // TODO: should we expose the `get_account_update` endpoint from the Client?
    let (endpoint, timeout, ..) = get_client_config();
    let rpc_api = TonicRpcClient::new(&endpoint, timeout);
    let details1 = rpc_api.get_account_details(basic_wallet_1.id()).await.unwrap();
    let details2 = rpc_api.get_account_details(basic_wallet_2.id()).await.unwrap();

    assert!(matches!(details1, AccountDetails::Private(_, _)));
    assert!(matches!(details2, AccountDetails::Public(_, _)));
}

#[tokio::test]
async fn test_sync_detail_values() {
    let (mut client1, authenticator_1) = create_test_client().await;
    let (mut client2, authenticator_2) = create_test_client().await;
    wait_for_node(&mut client1).await;
    wait_for_node(&mut client2).await;

    let (first_regular_account, faucet_account_header) =
        setup_wallet_and_faucet(&mut client1, AccountStorageMode::Private, &authenticator_1).await;

    let (second_regular_account, ..) =
        insert_new_wallet(&mut client2, AccountStorageMode::Private, &authenticator_2)
            .await
            .unwrap();

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    // First Mint necessary token
    mint_and_consume(&mut client1, from_account_id, faucet_account_id, NoteType::Private).await;

    // Second client sync shouldn't have any new changes
    let new_details = client2.sync_state().await.unwrap();
    assert!(new_details.is_empty());

    // Do a transfer with recall from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        Some(new_details.block_num + 5),
        NoteType::Public,
        client1.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    let note_id = tx_request.expected_output_notes().next().unwrap().id();
    execute_tx_and_sync(&mut client1, from_account_id, tx_request).await;

    // Second client sync should have new note
    let new_details = client2.sync_state().await.unwrap();
    assert_eq!(new_details.committed_notes.len(), 1);
    assert_eq!(new_details.consumed_notes.len(), 0);
    assert_eq!(new_details.updated_accounts.len(), 0);

    // Consume the note with the second account
    let tx_request = TransactionRequestBuilder::consume_notes(vec![note_id]).build().unwrap();
    execute_tx_and_sync(&mut client2, to_account_id, tx_request).await;

    // First client sync should have a new nullifier as the note was consumed
    let new_details = client1.sync_state().await.unwrap();
    assert_eq!(new_details.committed_notes.len(), 0);
    assert_eq!(new_details.consumed_notes.len(), 1);
}

/// This test runs 3 mint transactions that get included in different blocks so that once we sync
/// we can check that each transaction gets marked as committed in the corresponding block.
#[tokio::test]
async fn test_multiple_transactions_can_be_committed_in_different_blocks_without_sync() {
    let (mut client, authenticator) = create_test_client().await;

    let (first_regular_account, faucet_account_header) =
        setup_wallet_and_faucet(&mut client, AccountStorageMode::Private, &authenticator).await;

    let from_account_id = first_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    // Mint first note
    let (first_note_id, first_note_tx_id) = {
        // Create a Mint Tx for 1000 units of our fungible asset
        let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();

        println!("Minting Asset");
        let tx_request = TransactionRequestBuilder::mint_fungible_asset(
            fungible_asset,
            from_account_id,
            NoteType::Private,
            client.rng(),
        )
        .unwrap()
        .build()
        .unwrap();

        println!("Executing transaction...");
        let transaction_execution_result =
            client.new_transaction(faucet_account_id, tx_request.clone()).await.unwrap();
        let transaction_id = transaction_execution_result.executed_transaction().id();

        println!("Sending transaction to node");
        let note_id = tx_request.expected_output_notes().next().unwrap().id();
        client.submit_transaction(transaction_execution_result).await.unwrap();

        (note_id, transaction_id)
    };

    // Mint second note
    let (second_note_id, second_note_tx_id) = {
        // Create a Mint Tx for 1000 units of our fungible asset
        let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();

        println!("Minting Asset");
        let tx_request = TransactionRequestBuilder::mint_fungible_asset(
            fungible_asset,
            from_account_id,
            NoteType::Private,
            client.rng(),
        )
        .unwrap()
        .build()
        .unwrap();

        println!("Executing transaction...");
        let transaction_execution_result =
            client.new_transaction(faucet_account_id, tx_request.clone()).await.unwrap();
        let transaction_id = transaction_execution_result.executed_transaction().id();

        println!("Sending transaction to node");
        // May need a few attempts until it gets included
        let note_id = tx_request.expected_output_notes().next().unwrap().id();
        while client
            .test_rpc_api()
            .get_notes_by_id(&[first_note_id])
            .await
            .unwrap()
            .is_empty()
        {
            std::thread::sleep(Duration::from_secs(3));
        }
        client.submit_transaction(transaction_execution_result).await.unwrap();

        (note_id, transaction_id)
    };

    // Mint third note
    let (third_note_id, third_note_tx_id) = {
        // Create a Mint Tx for 1000 units of our fungible asset
        let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();

        println!("Minting Asset");
        let tx_request = TransactionRequestBuilder::mint_fungible_asset(
            fungible_asset,
            from_account_id,
            NoteType::Private,
            client.rng(),
        )
        .unwrap()
        .build()
        .unwrap();

        println!("Executing transaction...");
        let transaction_execution_result =
            client.new_transaction(faucet_account_id, tx_request.clone()).await.unwrap();
        let transaction_id = transaction_execution_result.executed_transaction().id();

        println!("Sending transaction to node");
        // May need a few attempts until it gets included
        let note_id = tx_request.expected_output_notes().next().unwrap().id();
        while client
            .test_rpc_api()
            .get_notes_by_id(&[second_note_id])
            .await
            .unwrap()
            .is_empty()
        {
            std::thread::sleep(Duration::from_secs(3));
        }
        client.submit_transaction(transaction_execution_result).await.unwrap();

        (note_id, transaction_id)
    };

    // Wait until the note gets comitted in the node (without syncing)
    while client
        .test_rpc_api()
        .get_notes_by_id(&[third_note_id])
        .await
        .unwrap()
        .is_empty()
    {
        std::thread::sleep(Duration::from_secs(3));
    }

    client.sync_state().await.unwrap();

    let all_transactions = client.get_transactions(TransactionFilter::All).await.unwrap();
    let first_tx = all_transactions.iter().find(|tx| tx.id == first_note_tx_id).unwrap();
    let second_tx = all_transactions.iter().find(|tx| tx.id == second_note_tx_id).unwrap();
    let third_tx = all_transactions.iter().find(|tx| tx.id == third_note_tx_id).unwrap();

    match (
        first_tx.transaction_status.clone(),
        second_tx.transaction_status.clone(),
        third_tx.transaction_status.clone(),
    ) {
        (
            TransactionStatus::Committed(first_tx_commit_height),
            TransactionStatus::Committed(second_tx_commit_height),
            TransactionStatus::Committed(third_tx_commit_height),
        ) => {
            assert!(first_tx_commit_height < second_tx_commit_height);
            assert!(second_tx_commit_height < third_tx_commit_height);
        },
        _ => {
            panic!("All three TXs should be committed in different blocks")
        },
    }
}

/// Test that checks multiple features:
/// - Consuming multiple notes in a single transaction.
/// - Consuming authenticated notes.
/// - Consuming unauthenticated notes.
#[tokio::test]
async fn test_consume_multiple_expected_notes() {
    let (mut client, authenticator_1) = create_test_client().await;
    let (mut unauth_client, authenticator_2) = create_test_client().await;

    wait_for_node(&mut client).await;

    // Setup accounts
    let (target_basic_account_1, faucet_account_header) =
        setup_wallet_and_faucet(&mut client, AccountStorageMode::Private, &authenticator_1).await;
    let (target_basic_account_2, ..) =
        insert_new_wallet(&mut unauth_client, AccountStorageMode::Private, &authenticator_2)
            .await
            .unwrap();
    unauth_client.sync_state().await.unwrap();

    let faucet_account_id = faucet_account_header.id();
    let to_account_ids = [target_basic_account_1.id(), target_basic_account_2.id()];

    // Mint tokens to the accounts
    let fungible_asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let mint_tx_request = mint_multiple_fungible_asset(
        fungible_asset,
        vec![to_account_ids[0], to_account_ids[0], to_account_ids[1], to_account_ids[1]],
        NoteType::Private,
        client.rng(),
    );

    execute_tx_and_sync(&mut client, faucet_account_id, mint_tx_request.clone()).await;
    unauth_client.sync_state().await.unwrap();

    // Filter notes by ownership
    let expected_notes = mint_tx_request.expected_output_notes();
    let client_notes: Vec<_> = client.get_input_notes(NoteFilter::All).await.unwrap();
    let client_notes_ids: Vec<_> = client_notes.iter().map(|note| note.id()).collect();

    let (client_owned_notes, unauth_owned_notes): (Vec<_>, Vec<_>) =
        expected_notes.partition(|note| client_notes_ids.contains(&note.id()));

    // Create and execute transactions
    let tx_request_1 = TransactionRequestBuilder::consume_notes(
        client_owned_notes.iter().map(|note| note.id()).collect(),
    )
    .with_authenticated_input_notes(client_owned_notes.iter().map(|note| (note.id(), None)))
    .build()
    .unwrap();

    let tx_request_2 = TransactionRequestBuilder::consume_notes(
        unauth_owned_notes.iter().map(|note| note.id()).collect(),
    )
    .with_unauthenticated_input_notes(unauth_owned_notes.iter().map(|note| ((*note).clone(), None)))
    .build()
    .unwrap();

    let tx_id_1 = execute_tx(&mut client, to_account_ids[0], tx_request_1).await;
    let tx_id_2 = execute_tx(&mut unauth_client, to_account_ids[1], tx_request_2).await;

    // Ensure notes are processed
    assert!(!client.get_input_notes(NoteFilter::Processing).await.unwrap().is_empty());
    assert!(!unauth_client.get_input_notes(NoteFilter::Processing).await.unwrap().is_empty());

    wait_for_tx(&mut client, tx_id_1).await;
    wait_for_tx(&mut unauth_client, tx_id_2).await;

    // Verify no remaining expected notes and all notes are consumed
    assert!(client.get_input_notes(NoteFilter::Expected).await.unwrap().is_empty());
    assert!(unauth_client.get_input_notes(NoteFilter::Expected).await.unwrap().is_empty());

    assert!(
        !client.get_input_notes(NoteFilter::Consumed).await.unwrap().is_empty(),
        "Authenticated notes are consumed"
    );
    assert!(
        !unauth_client.get_input_notes(NoteFilter::Consumed).await.unwrap().is_empty(),
        "Unauthenticated notes are consumed"
    );

    // Validate the final asset amounts in each account
    for (client, account_id) in
        vec![(client, to_account_ids[0]), (unauth_client, to_account_ids[1])]
    {
        assert_account_has_single_asset(
            &client,
            account_id,
            faucet_account_id,
            TRANSFER_AMOUNT * 2,
        )
        .await;
    }
}

#[tokio::test]
async fn test_import_consumed_note_with_proof() {
    let (mut client_1, authenticator_1) = create_test_client().await;
    let (first_regular_account, faucet_account_header) =
        setup_wallet_and_faucet(&mut client_1, AccountStorageMode::Private, &authenticator_1).await;

    let (mut client_2, authenticator_2) = create_test_client().await;
    let (client_2_account, _seed, _) =
        insert_new_wallet(&mut client_2, AccountStorageMode::Private, &authenticator_2)
            .await
            .unwrap();

    wait_for_node(&mut client_2).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = client_2_account.id();
    let faucet_account_id = faucet_account_header.id();

    mint_and_consume(&mut client_1, from_account_id, faucet_account_id, NoteType::Private).await;

    let current_block_num = client_1.get_sync_height().await.unwrap();
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();

    println!("Running P2IDR tx...");
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        Some(current_block_num),
        NoteType::Private,
        client_1.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    execute_tx_and_sync(&mut client_1, from_account_id, tx_request).await;
    let note = client_1
        .get_input_notes(NoteFilter::Committed)
        .await
        .unwrap()
        .first()
        .unwrap()
        .clone();

    // Consume the note with the sender account

    println!("Consuming Note...");
    let tx_request = TransactionRequestBuilder::consume_notes(vec![note.id()]).build().unwrap();
    execute_tx_and_sync(&mut client_1, from_account_id, tx_request).await;

    // Import the consumed note
    client_2
        .import_note(NoteFile::NoteWithProof(
            note.clone().try_into().unwrap(),
            note.inclusion_proof().unwrap().clone(),
        ))
        .await
        .unwrap();

    let consumed_note = client_2.get_input_note(note.id()).await.unwrap().unwrap();
    assert!(matches!(consumed_note.state(), InputNoteState::ConsumedExternal { .. }));
}

#[tokio::test]
async fn test_import_consumed_note_with_id() {
    let (mut client_1, authenticator) = create_test_client().await;
    let (first_regular_account, second_regular_account, faucet_account_header) =
        setup_two_wallets_and_faucet(&mut client_1, AccountStorageMode::Private, &authenticator)
            .await;

    let (mut client_2, _) = create_test_client().await;

    wait_for_node(&mut client_2).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    mint_and_consume(&mut client_1, from_account_id, faucet_account_id, NoteType::Private).await;

    let current_block_num = client_1.get_sync_height().await.unwrap();
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();

    println!("Running P2IDR tx...");
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        Some(current_block_num),
        NoteType::Public,
        client_1.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    execute_tx_and_sync(&mut client_1, from_account_id, tx_request).await;
    let note = client_1
        .get_input_notes(NoteFilter::Committed)
        .await
        .unwrap()
        .first()
        .unwrap()
        .clone();

    // Consume the note with the sender account

    println!("Consuming Note...");
    let tx_request = TransactionRequestBuilder::consume_notes(vec![note.id()]).build().unwrap();
    execute_tx_and_sync(&mut client_1, from_account_id, tx_request).await;
    client_2.sync_state().await.unwrap();

    // Import the consumed note
    client_2.import_note(NoteFile::NoteId(note.id())).await.unwrap();

    let consumed_note = client_2.get_input_note(note.id()).await.unwrap().unwrap();
    assert!(matches!(consumed_note.state(), InputNoteState::ConsumedExternal { .. }));
}

#[tokio::test]
async fn test_discarded_transaction() {
    let (mut client_1, authenticator_1) = create_test_client().await;
    let (first_regular_account, faucet_account_header) =
        setup_wallet_and_faucet(&mut client_1, AccountStorageMode::Private, &authenticator_1).await;

    let (mut client_2, authenticator_2) = create_test_client().await;
    let (second_regular_account, ..) =
        insert_new_wallet(&mut client_2, AccountStorageMode::Private, &authenticator_2)
            .await
            .unwrap();

    wait_for_node(&mut client_2).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    mint_and_consume(&mut client_1, from_account_id, faucet_account_id, NoteType::Private).await;

    let current_block_num = client_1.get_sync_height().await.unwrap();
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();

    println!("Running P2IDR tx...");
    let tx_request = TransactionRequestBuilder::pay_to_id(
        PaymentTransactionData::new(vec![Asset::Fungible(asset)], from_account_id, to_account_id),
        Some(current_block_num),
        NoteType::Public,
        client_1.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    execute_tx_and_sync(&mut client_1, from_account_id, tx_request).await;
    client_2.sync_state().await.unwrap();
    let note = client_1
        .get_input_notes(NoteFilter::Committed)
        .await
        .unwrap()
        .first()
        .unwrap()
        .clone();

    println!("Consuming Note...");
    let tx_request = TransactionRequestBuilder::consume_notes(vec![note.id()]).build().unwrap();

    // Consume the note in client 1 but dont submit it to the node
    let tx_result = client_1.new_transaction(from_account_id, tx_request.clone()).await.unwrap();
    let tx_id = tx_result.executed_transaction().id();
    client_1.testing_prove_transaction(&tx_result).await.unwrap();

    // Store the account state before applying the transaction
    let account_before_tx = client_1.get_account(from_account_id).await.unwrap().unwrap();
    let account_hash_before_tx = account_before_tx.account().commitment();

    // Apply the transaction
    client_1.testing_apply_transaction(tx_result).await.unwrap();

    // Check that the account state has changed after applying the transaction
    let account_after_tx = client_1.get_account(from_account_id).await.unwrap().unwrap();
    let account_hash_after_tx = account_after_tx.account().commitment();

    assert_ne!(
        account_hash_before_tx, account_hash_after_tx,
        "Account hash should change after applying the transaction"
    );

    let note_record = client_1.get_input_note(note.id()).await.unwrap().unwrap();
    assert!(matches!(note_record.state(), InputNoteState::ProcessingAuthenticated(_)));

    // Consume the note in client 2
    execute_tx_and_sync(&mut client_2, to_account_id, tx_request).await;

    let note_record = client_2.get_input_note(note.id()).await.unwrap().unwrap();
    assert!(matches!(note_record.state(), InputNoteState::ConsumedAuthenticatedLocal(_)));

    // After sync the note in client 1 should be consumed externally and the transaction discarded
    client_1.sync_state().await.unwrap();
    let note_record = client_1.get_input_note(note.id()).await.unwrap().unwrap();
    assert!(matches!(note_record.state(), InputNoteState::ConsumedExternal(_)));
    let tx_record = client_1
        .get_transactions(TransactionFilter::All)
        .await
        .unwrap()
        .into_iter()
        .find(|tx| tx.id == tx_id)
        .unwrap();
    assert!(matches!(tx_record.transaction_status, TransactionStatus::Discarded));

    // Check that the account state has been rolled back after the transaction was discarded
    let account_after_sync = client_1.get_account(from_account_id).await.unwrap().unwrap();
    let account_hash_after_sync = account_after_sync.account().commitment();

    assert_ne!(
        account_hash_after_sync, account_hash_after_tx,
        "Account hash should change after transaction was discarded"
    );
    assert_eq!(
        account_hash_after_sync, account_hash_before_tx,
        "Account hash should be rolled back to the value before the transaction"
    );
}

struct AlwaysFailingProver;

impl AlwaysFailingProver {
    pub fn new() -> Self {
        Self
    }
}

#[maybe_async_trait]
impl TransactionProver for AlwaysFailingProver {
    #[maybe_async]
    fn prove(
        &self,
        _tx_witness: TransactionWitness,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        return Err(TransactionProverError::other("This prover always fails"));
    }
}

#[tokio::test]
async fn test_custom_transaction_prover() {
    let (mut client, authenticator) = create_test_client().await;
    let (first_regular_account, faucet_account_header) =
        setup_wallet_and_faucet(&mut client, AccountStorageMode::Private, &authenticator).await;

    let from_account_id = first_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();

    let tx_request = TransactionRequestBuilder::mint_fungible_asset(
        fungible_asset,
        from_account_id,
        NoteType::Private,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();

    let transaction_execution_result =
        client.new_transaction(faucet_account_id, tx_request.clone()).await.unwrap();

    let result = client
        .submit_transaction_with_prover(
            transaction_execution_result,
            Arc::new(AlwaysFailingProver::new()),
        )
        .await;

    assert!(matches!(
        result,
        Err(ClientError::TransactionProvingError(TransactionProverError::Other {
            error_msg: _,
            source: _
        }))
    ));
}

#[tokio::test]
async fn test_locked_account() {
    let (mut client_1, authenticator) = create_test_client().await;

    let (faucet_account, ..) =
        insert_new_fungible_faucet(&mut client_1, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap();

    let (private_account, seed, _) =
        insert_new_wallet(&mut client_1, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap();

    let from_account_id = private_account.id();
    let faucet_account_id = faucet_account.id();

    wait_for_node(&mut client_1).await;

    mint_and_consume(&mut client_1, from_account_id, faucet_account_id, NoteType::Private).await;

    let private_account = client_1.get_account(from_account_id).await.unwrap().unwrap().into();

    // Import private account in client 2
    let (mut client_2, _) = create_test_client().await;
    client_2.add_account(&private_account, seed.into(), false).await.unwrap();

    wait_for_node(&mut client_2).await;

    // When imported the account shouldn't be locked
    let account_record = client_2.get_account(from_account_id).await.unwrap().unwrap();
    assert!(!account_record.is_locked());

    // Consume note with private account in client 1
    mint_and_consume(&mut client_1, from_account_id, faucet_account_id, NoteType::Private).await;

    // After sync the private account should be locked in client 2
    let summary = client_2.sync_state().await.unwrap();
    assert!(summary.locked_accounts.contains(&from_account_id));
    let account_record = client_2.get_account(from_account_id).await.unwrap().unwrap();
    assert!(account_record.is_locked());

    // Get updated account from client 1 and import it in client 2 with `overwrite` flag
    let updated_private_account =
        client_1.get_account(from_account_id).await.unwrap().unwrap().into();
    client_2.add_account(&updated_private_account, None, true).await.unwrap();

    // After sync the private account shouldn't be locked in client 2
    client_2.sync_state().await.unwrap();
    let account_record = client_2.get_account(from_account_id).await.unwrap().unwrap();
    assert!(!account_record.is_locked());
}

#[tokio::test]
async fn test_expired_transaction_fails() {
    let (mut client, authenticator) = create_test_client().await;
    let (faucet_account, ..) =
        insert_new_fungible_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap();

    let (private_account, ..) =
        insert_new_wallet(&mut client, AccountStorageMode::Private, &authenticator)
            .await
            .unwrap();

    let from_account_id = private_account.id();
    let faucet_account_id = faucet_account.id();

    wait_for_node(&mut client).await;

    let expiration_delta = 2;

    // Create a Mint Tx for 1000 units of our fungible asset
    let fungible_asset = FungibleAsset::new(faucet_account_id, MINT_AMOUNT).unwrap();
    println!("Minting Asset");
    let tx_request = TransactionRequestBuilder::mint_fungible_asset(
        fungible_asset,
        from_account_id,
        NoteType::Public,
        client.rng(),
    )
    .unwrap()
    .with_expiration_delta(expiration_delta)
    .build()
    .unwrap();

    println!("Executing transaction...");
    let transaction_execution_result =
        client.new_transaction(faucet_account_id, tx_request).await.unwrap();

    println!("Transaction executed successfully");
    wait_for_blocks(&mut client, (expiration_delta + 1).into()).await;

    println!("Sending transaction to node");
    let submited_tx_result = client.submit_transaction(transaction_execution_result).await;

    assert!(submited_tx_result.is_err());
}

/// Tests that RPC methods that are not directly related to the client logic
/// (like GetBlockByNumber) work correctly
#[tokio::test]
async fn test_unused_rpc_api() {
    let (mut client, keystore) = create_test_client().await;

    let (first_basic_account, faucet_account) =
        setup_wallet_and_faucet(&mut client, AccountStorageMode::Public, &keystore).await;

    wait_for_node(&mut client).await;
    client.sync_state().await.unwrap();

    let first_block_num = client.get_sync_height().await.unwrap();

    let (block_header, _) = client
        .test_rpc_api()
        .get_block_header_by_number(Some(first_block_num), false)
        .await
        .unwrap();
    let block = client.test_rpc_api().get_block_by_number(first_block_num).await.unwrap();

    assert_eq!(&block_header, block.header());

    let note =
        mint_note(&mut client, first_basic_account.id(), faucet_account.id(), NoteType::Public)
            .await;

    consume_notes(&mut client, first_basic_account.id(), &[note.clone()]).await;

    client.sync_state().await.unwrap();

    let second_block_num = client.get_sync_height().await.unwrap();

    let nullifier = note.nullifier();

    let node_nullifier = client
        .test_rpc_api()
        .check_nullifiers_by_prefix(&[nullifier.prefix()], 0.into())
        .await
        .unwrap()
        .pop()
        .unwrap();
    let node_nullifier_proof = client
        .test_rpc_api()
        .check_nullifiers(&[nullifier])
        .await
        .unwrap()
        .pop()
        .unwrap();

    assert_eq!(node_nullifier.nullifier, nullifier);
    assert_eq!(node_nullifier_proof.leaf().entries().pop().unwrap().0, nullifier.inner());

    let account_delta = client
        .test_rpc_api()
        .get_account_state_delta(first_basic_account.id(), first_block_num, second_block_num)
        .await
        .unwrap();

    assert_eq!(account_delta.nonce(), Some(ONE));
    assert_eq!(*account_delta.vault().fungible().iter().next().unwrap().1, MINT_AMOUNT as i64);
}

#[tokio::test]
async fn test_stale_transactions_discarded() {
    let (mut client, authenticator) = create_test_client().await;
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
    wait_for_blocks(&mut client, TX_GRACEFUL_BLOCKS + 1).await;

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

#[tokio::test]
async fn test_ignore_invalid_notes() {
    let (mut client, authenticator) = create_test_client().await;
    let (regular_account, second_regular_account, faucet_account_header) =
        setup_two_wallets_and_faucet(&mut client, AccountStorageMode::Private, &authenticator)
            .await;

    let account_id = regular_account.id();
    let second_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_header.id();

    // Mint 2 valid notes
    let note_1 = mint_note(&mut client, account_id, faucet_account_id, NoteType::Private).await;
    let note_2 = mint_note(&mut client, account_id, faucet_account_id, NoteType::Private).await;

    // Mint 2 invalid notes
    let note_3 =
        mint_note(&mut client, second_account_id, faucet_account_id, NoteType::Private).await;
    let note_4 =
        mint_note(&mut client, second_account_id, faucet_account_id, NoteType::Private).await;

    // Create a transaction to consume all 4 notes but ignore the invalid ones
    let tx_request = TransactionRequestBuilder::consume_notes(vec![
        note_1.id(),
        note_2.id(),
        note_3.id(),
        note_4.id(),
    ])
    .ignore_invalid_input_notes()
    .build()
    .unwrap();

    execute_tx_and_sync(&mut client, account_id, tx_request).await;

    // Check that only the valid notes were consumed
    let consumed_notes = client.get_input_notes(NoteFilter::Consumed).await.unwrap();
    assert_eq!(consumed_notes.len(), 2);
    assert!(consumed_notes.iter().any(|note| note.id() == note_1.id()));
    assert!(consumed_notes.iter().any(|note| note.id() == note_2.id()));
}
