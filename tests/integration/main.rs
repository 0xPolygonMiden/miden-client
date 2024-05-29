use miden_client::{
    client::{
        accounts::AccountTemplate,
        rpc::{AccountDetails, NodeRpcClient},
        transactions::transaction_request::{PaymentTransactionData, TransactionTemplate},
        NoteRelevance,
    },
    errors::ClientError,
    store::{NoteFilter, NoteStatus},
};
use miden_objects::{
    accounts::{AccountId, AccountStorageType},
    assets::{Asset, FungibleAsset},
    notes::NoteType,
};
use miden_tx::TransactionExecutorError;

mod common;
use common::*;

mod custom_transactions_tests;
mod onchain_tests;
mod swap_transactions_tests;

#[tokio::test]
async fn test_added_notes() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let (_, _, faucet_account_stub) = setup(&mut client, AccountStorageType::OffChain).await;
    // Mint some asset for an account not tracked by the client. It should not be stored as an
    // input note afterwards since it is not being tracked by the client
    let fungible_asset = FungibleAsset::new(faucet_account_stub.id(), MINT_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::MintFungibleAsset(
        fungible_asset,
        AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap(),
        NoteType::OffChain,
    );
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    println!("Running Mint tx...");
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that no new notes were added
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(notes.is_empty())
}

#[tokio::test]
async fn test_p2id_transfer() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client, AccountStorageType::OffChain).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;
    consume_notes(&mut client, from_account_id, &[note]).await;
    assert_account_has_single_asset(&client, from_account_id, faucet_account_id, MINT_AMOUNT).await;

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        NoteType::OffChain,
    );
    println!("Running P2ID tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that note is committed for the second account to consume
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    // Consume P2ID note
    let tx_template = TransactionTemplate::ConsumeNotes(to_account_id, vec![notes[0].id()]);
    println!("Consuming Note...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Ensure we have nothing else to consume
    let current_notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(current_notes.is_empty());

    let (regular_account, seed) = client.get_account(from_account_id).unwrap();

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

    let (regular_account, _seed) = client.get_account(to_account_id).unwrap();
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
async fn test_p2idr_transfer_consumed_by_target() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client, AccountStorageType::OffChain).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;
    println!("about to consume");

    //Check that the note is not consumed by the target account
    assert!(matches!(
        client.get_input_note(note.id()).unwrap().status(),
        NoteStatus::Committed
    ));

    consume_notes(&mut client, from_account_id, &[note.clone()]).await;
    assert_account_has_single_asset(&client, from_account_id, faucet_account_id, MINT_AMOUNT).await;

    // Check that the note is consumed by the target account
    let input_note = client.get_input_note(note.id()).unwrap();
    assert!(matches!(input_note.status(), NoteStatus::Consumed));
    assert_eq!(input_note.consumer_account_id().unwrap(), from_account_id);

    // Do a transfer from first account to second account with Recall. In this situation we'll do
    // the happy path where the `to_account_id` consumes the note
    println!("getting balance");
    let from_account_balance = client
        .get_account(from_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let to_account_balance = client
        .get_account(to_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let current_block_num = client.get_sync_height().unwrap();
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToIdWithRecall(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        current_block_num + 50,
        NoteType::OffChain,
    );
    println!("Running P2IDR tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that note is committed for the second account to consume
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    // Make the `to_account_id` consume P2IDR note
    let tx_template = TransactionTemplate::ConsumeNotes(to_account_id, vec![notes[0].id()]);
    println!("Consuming Note...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    let (regular_account, seed) = client.get_account(from_account_id).unwrap();
    // The seed should not be retrieved due to the account not being new
    assert!(!regular_account.is_new() && seed.is_none());
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    // Validate the transfered amounts
    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), from_account_balance - TRANSFER_AMOUNT);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    let (regular_account, _seed) = client.get_account(to_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), to_account_balance + TRANSFER_AMOUNT);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    assert_note_cannot_be_consumed_twice(&mut client, to_account_id, notes[0].id()).await;
}

#[tokio::test]
async fn test_p2idr_transfer_consumed_by_sender() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client, AccountStorageType::OffChain).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;

    consume_notes(&mut client, from_account_id, &[note]).await;
    assert_account_has_single_asset(&client, from_account_id, faucet_account_id, MINT_AMOUNT).await;
    // Do a transfer from first account to second account with Recall. In this situation we'll do
    // the happy path where the `to_account_id` consumes the note
    let from_account_balance = client
        .get_account(from_account_id)
        .unwrap()
        .0
        .vault()
        .get_balance(faucet_account_id)
        .unwrap_or(0);
    let current_block_num = client.get_sync_height().unwrap();
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToIdWithRecall(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        current_block_num + 5,
        NoteType::OffChain,
    );
    println!("Running P2IDR tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that note is committed
    println!("Fetching Committed Notes...");
    let notes = client.get_input_notes(NoteFilter::Committed).unwrap();
    assert!(!notes.is_empty());

    // Check that it's still too early to consume
    let tx_template = TransactionTemplate::ConsumeNotes(from_account_id, vec![notes[0].id()]);
    println!("Consuming Note (too early)...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    let transaction_execution_result = client.new_transaction(tx_request);
    assert!(transaction_execution_result.is_err_and(|err| {
        matches!(
            err,
            ClientError::TransactionExecutorError(
                TransactionExecutorError::ExecuteTransactionProgramFailed(_)
            )
        )
    }));

    // Wait to consume with the sender account
    println!("Waiting for note to be consumable by sender");
    let current_block_num = client.get_sync_height().unwrap();

    while client.get_sync_height().unwrap() < current_block_num + 5 {
        client.sync_state().await.unwrap();
    }

    // Consume the note with the sender account
    let tx_template = TransactionTemplate::ConsumeNotes(from_account_id, vec![notes[0].id()]);
    println!("Consuming Note...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    let (regular_account, seed) = client.get_account(from_account_id).unwrap();
    // The seed should not be retrieved due to the account not being new
    assert!(!regular_account.is_new() && seed.is_none());
    assert_eq!(regular_account.vault().assets().count(), 1);
    let asset = regular_account.vault().assets().next().unwrap();

    // Validate the the sender hasn't lost funds
    if let Asset::Fungible(fungible_asset) = asset {
        assert_eq!(fungible_asset.amount(), from_account_balance);
    } else {
        panic!("Error: Account should have a fungible asset");
    }

    let (regular_account, _seed) = client.get_account(to_account_id).unwrap();
    assert_eq!(regular_account.vault().assets().count(), 0);

    // Check that the target can't consume the note anymore
    assert_note_cannot_be_consumed_twice(&mut client, to_account_id, notes[0].id()).await;
}

#[tokio::test]
async fn test_get_consumable_notes() {
    let mut client = create_test_client();

    let (first_regular_account, second_regular_account, faucet_account_stub) =
        setup(&mut client, AccountStorageType::OffChain).await;

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    //No consumable notes initially
    assert!(client.get_consumable_notes(None).unwrap().is_empty());

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;

    // Check that note is consumable by the account that minted
    assert!(!client.get_consumable_notes(None).unwrap().is_empty());
    assert!(!client.get_consumable_notes(Some(from_account_id)).unwrap().is_empty());
    assert!(client.get_consumable_notes(Some(to_account_id)).unwrap().is_empty());

    consume_notes(&mut client, from_account_id, &[note]).await;

    //After consuming there are no more consumable notes
    assert!(client.get_consumable_notes(None).unwrap().is_empty());

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToIdWithRecall(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        100,
        NoteType::OffChain,
    );
    println!("Running P2IDR tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client, tx_request).await;

    // Check that note is consumable by both accounts
    let consumable_notes = client.get_consumable_notes(None).unwrap();
    let relevant_accounts = &consumable_notes.first().unwrap().relevances;
    assert_eq!(relevant_accounts.len(), 2);
    assert!(!client.get_consumable_notes(Some(from_account_id)).unwrap().is_empty());
    assert!(!client.get_consumable_notes(Some(to_account_id)).unwrap().is_empty());

    // Check that the note is only consumable after block 100 for the account that sent the transaction
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
    let mut client = create_test_client();

    let (first_regular_account, _, faucet_account_stub) =
        setup(&mut client, AccountStorageType::OffChain).await;

    let from_account_id = first_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();
    let random_account_id = AccountId::from_hex("0x0123456789abcdef").unwrap();

    // No output notes initially
    assert!(client.get_output_notes(NoteFilter::All).unwrap().is_empty());

    // First Mint necesary token
    let note = mint_note(&mut client, from_account_id, faucet_account_id, NoteType::OffChain).await;

    // Check that there was an output note but it wasn't consumed
    assert!(client.get_output_notes(NoteFilter::Consumed).unwrap().is_empty());
    assert!(!client.get_output_notes(NoteFilter::All).unwrap().is_empty());

    consume_notes(&mut client, from_account_id, &[note]).await;

    //After consuming, the note is returned when using the [NoteFilter::Consumed] filter
    assert!(!client.get_output_notes(NoteFilter::Consumed).unwrap().is_empty());

    // Do a transfer from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToId(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, random_account_id),
        NoteType::OffChain,
    );
    println!("Running P2ID tx...");
    let tx_request = client.build_transaction_request(tx_template).unwrap();

    let output_note_id = tx_request.expected_output_notes()[0].id();

    // Before executing, the output note is not found
    assert!(client.get_output_note(output_note_id).is_err());

    execute_tx_and_sync(&mut client, tx_request).await;

    // After executing, the note is only found in output notes
    assert!(client.get_output_note(output_note_id).is_ok());
    assert!(client.get_input_note(output_note_id).is_err());
}

#[tokio::test]
async fn test_import_pending_notes() {
    let mut client_1 = create_test_client();
    let (first_basic_account, _second_basic_account, faucet_account) =
        setup(&mut client_1, AccountStorageType::OffChain).await;

    let mut client_2 = create_test_client();
    let (client_2_account, _seed) = client_2
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: true,
            storage_type: AccountStorageType::OffChain,
        })
        .unwrap();

    wait_for_node(&mut client_2).await;

    let tx_template = TransactionTemplate::MintFungibleAsset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        client_2_account.id(),
        NoteType::OffChain,
    );

    let tx_request = client_1.build_transaction_request(tx_template).unwrap();
    let note = tx_request.expected_output_notes()[0].clone();
    client_2.sync_state().await.unwrap();

    // If the verification is requested before execution then the import should fail
    assert!(client_2.import_input_note(note.clone().into(), true).await.is_err());
    execute_tx_and_sync(&mut client_1, tx_request).await;

    // Use client 1 to wait until a couple of blocks have passed
    wait_for_blocks(&mut client_1, 3).await;

    let new_sync_data = client_2.sync_state().await.unwrap();
    client_2.import_input_note(note.clone().into(), true).await.unwrap();
    let input_note = client_2.get_input_note(note.id()).unwrap();
    assert!(new_sync_data.block_num > input_note.inclusion_proof().unwrap().origin().block_num + 1);

    // If imported after execution and syncing then the inclusion proof should be Some
    assert!(input_note.inclusion_proof().is_some());

    // If client 2 succesfully consumes the note, we confirm we have MMR and block header data
    consume_notes(&mut client_2, client_2_account.id(), &[input_note.try_into().unwrap()]).await;

    let tx_template = TransactionTemplate::MintFungibleAsset(
        FungibleAsset::new(faucet_account.id(), MINT_AMOUNT).unwrap(),
        first_basic_account.id(),
        NoteType::OffChain,
    );

    let tx_request = client_1.build_transaction_request(tx_template).unwrap();
    let note = tx_request.expected_output_notes()[0].clone();

    // Import an uncommited note without verification
    client_2.import_input_note(note.clone().into(), false).await.unwrap();
    let input_note = client_2.get_input_note(note.id()).unwrap();

    // If imported before execution then the inclusion proof should be None
    assert!(input_note.inclusion_proof().is_none());

    execute_tx_and_sync(&mut client_1, tx_request).await;
    client_2.sync_state().await.unwrap();

    // After sync, the imported note should have inclusion proof even if it's not relevant for its accounts.
    let input_note = client_2.get_input_note(note.id()).unwrap();
    assert!(input_note.inclusion_proof().is_some());

    // If inclusion proof is invalid this should panic
    consume_notes(&mut client_1, first_basic_account.id(), &[input_note.try_into().unwrap()]).await;
}

#[tokio::test]
async fn test_get_account_update() {
    // Create a client with both public and private accounts.
    let mut client = create_test_client();

    let (basic_wallet_1, _, faucet_account) =
        setup(&mut client, AccountStorageType::OffChain).await;

    let (basic_wallet_2, _) = client
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_type: AccountStorageType::OnChain,
        })
        .unwrap();

    // Mint and consume notes with both accounts so they are included in the node.
    let note1 =
        mint_note(&mut client, basic_wallet_1.id(), faucet_account.id(), NoteType::OffChain).await;
    let note2 =
        mint_note(&mut client, basic_wallet_2.id(), faucet_account.id(), NoteType::OffChain).await;

    client.sync_state().await.unwrap();

    consume_notes(&mut client, basic_wallet_1.id(), &[note1]).await;
    consume_notes(&mut client, basic_wallet_2.id(), &[note2]).await;

    wait_for_node(&mut client).await;
    client.sync_state().await.unwrap();

    // Request updates from node for both accounts. The request should not fail and both types of
    // [AccountDetails] should be received.
    let details1 = client.rpc_api().get_account_update(basic_wallet_1.id()).await.unwrap();
    let details2 = client.rpc_api().get_account_update(basic_wallet_2.id()).await.unwrap();

    assert!(matches!(details1, AccountDetails::OffChain(_, _)));
    assert!(matches!(details2, AccountDetails::Public(_, _)));
}

#[tokio::test]
async fn test_sync_detail_values() {
    let mut client1 = create_test_client();
    let mut client2 = create_test_client();
    wait_for_node(&mut client1).await;
    wait_for_node(&mut client2).await;

    let (first_regular_account, _, faucet_account_stub) =
        setup(&mut client1, AccountStorageType::OffChain).await;

    let (second_regular_account, _) = client2
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_type: AccountStorageType::OffChain,
        })
        .unwrap();

    let from_account_id = first_regular_account.id();
    let to_account_id = second_regular_account.id();
    let faucet_account_id = faucet_account_stub.id();

    // First Mint necesary token
    let note =
        mint_note(&mut client1, from_account_id, faucet_account_id, NoteType::OffChain).await;
    consume_notes(&mut client1, from_account_id, &[note]).await;
    assert_account_has_single_asset(&client1, from_account_id, faucet_account_id, MINT_AMOUNT)
        .await;

    // Second client sync shouldn't have any new changes
    let new_details = client2.sync_state().await.unwrap();
    assert!(new_details.is_empty());

    // Do a transfer with recall from first account to second account
    let asset = FungibleAsset::new(faucet_account_id, TRANSFER_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::PayToIdWithRecall(
        PaymentTransactionData::new(Asset::Fungible(asset), from_account_id, to_account_id),
        new_details.block_num + 5,
        NoteType::Public,
    );

    let tx_request = client1.build_transaction_request(tx_template).unwrap();
    let note_id = tx_request.expected_output_notes()[0].id();
    execute_tx_and_sync(&mut client1, tx_request).await;

    // Second client sync should have new note
    let new_details = client2.sync_state().await.unwrap();
    assert_eq!(new_details.new_notes, 1);
    assert_eq!(new_details.new_inclusion_proofs, 0);
    assert_eq!(new_details.new_nullifiers, 0);
    assert_eq!(new_details.updated_onchain_accounts, 0);

    // Consume the note with the second account
    let tx_template = TransactionTemplate::ConsumeNotes(to_account_id, vec![note_id]);
    let tx_request = client2.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client2, tx_request).await;

    // First client sync should have a new nullifier as the note was consumed
    let new_details = client1.sync_state().await.unwrap();
    assert_eq!(new_details.new_notes, 0);
    assert_eq!(new_details.new_inclusion_proofs, 0);
    assert_eq!(new_details.new_nullifiers, 1);
}
