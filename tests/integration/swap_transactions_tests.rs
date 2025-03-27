use miden_client::{
    account::Account,
    note::build_swap_tag,
    transaction::{OutputNote, TransactionRequestBuilder},
};
use miden_objects::{
    account::{AccountId, AccountStorageMode},
    asset::{Asset, FungibleAsset},
    note::{NoteId, NoteType},
};

use super::common::*;

// SWAP FULLY ONCHAIN
// ================================================================================================

#[tokio::test]
async fn test_swap_fully_onchain() {
    const OFFERED_ASSET_AMOUNT: u64 = 1;
    const REQUESTED_ASSET_AMOUNT: u64 = 25;
    const BTC_MINT_AMOUNT: u64 = 1000;
    const ETH_MINT_AMOUNT: u64 = 1000;
    let (mut client1, authenticator_1) = create_test_client().await;
    wait_for_node(&mut client1).await;
    let (mut client2, authenticator_2) = create_test_client().await;
    let (mut client_with_faucets, authenticator_3) = create_test_client().await;

    client1.sync_state().await.unwrap();
    client2.sync_state().await.unwrap();
    client_with_faucets.sync_state().await.unwrap();

    // Create Client 1's basic wallet (We'll call it accountA)
    let (account_a, ..) =
        insert_new_wallet(&mut client1, AccountStorageMode::Private, &authenticator_1)
            .await
            .unwrap();

    // Create Client 2's basic wallet (We'll call it accountB)
    let (account_b, ..) =
        insert_new_wallet(&mut client2, AccountStorageMode::Private, &authenticator_2)
            .await
            .unwrap();

    // Create client with faucets BTC faucet (note: it's not real BTC)
    let (btc_faucet_account, ..) = insert_new_fungible_faucet(
        &mut client_with_faucets,
        AccountStorageMode::Private,
        &authenticator_3,
    )
    .await
    .unwrap();

    // Create client with faucets ETH faucet (note: it's not real ETH)
    let (eth_faucet_account, ..) = insert_new_fungible_faucet(
        &mut client_with_faucets,
        AccountStorageMode::Private,
        &authenticator_3,
    )
    .await
    .unwrap();

    // mint 1000 BTC for accountA
    println!("minting 1000 btc for account A");
    let account_a_mint_note_id = mint(
        &mut client_with_faucets,
        account_a.id(),
        btc_faucet_account.id(),
        NoteType::Public,
        BTC_MINT_AMOUNT,
    )
    .await;
    // mint 1000 ETH for accountB
    println!("minting 1000 eth for account B");
    let account_b_mint_note_id = mint(
        &mut client_with_faucets,
        account_b.id(),
        eth_faucet_account.id(),
        NoteType::Public,
        ETH_MINT_AMOUNT,
    )
    .await;

    // Sync and consume note for accountA
    client1.sync_state().await.unwrap();
    let client_1_consumable_notes =
        client1.get_consumable_notes(Some(account_a.id())).await.unwrap();
    assert!(
        client_1_consumable_notes
            .iter()
            .any(|(note, _)| note.id() == account_a_mint_note_id)
    );

    println!("Consuming mint note on first client...");

    let tx_request = TransactionRequestBuilder::consume_notes(vec![account_a_mint_note_id])
        .build()
        .unwrap();
    execute_tx_and_sync(&mut client1, account_a.id(), tx_request).await;

    // Sync and consume note for accountB
    client2.sync_state().await.unwrap();
    let client_2_consumable_notes =
        client2.get_consumable_notes(Some(account_b.id())).await.unwrap();
    assert!(
        client_2_consumable_notes
            .iter()
            .any(|(note, _)| note.id() == account_b_mint_note_id)
    );

    println!("Consuming mint note on second client...");

    let tx_request = TransactionRequestBuilder::consume_notes(vec![account_b_mint_note_id])
        .build()
        .unwrap();
    execute_tx_and_sync(&mut client2, account_b.id(), tx_request).await;

    // Create ONCHAIN swap note (clientA offers 1 BTC in exchange of 25 ETH)
    // check that account now has 1 less BTC
    println!("creating swap note with accountA");
    let offered_asset = FungibleAsset::new(btc_faucet_account.id(), OFFERED_ASSET_AMOUNT).unwrap();
    let requested_asset =
        FungibleAsset::new(eth_faucet_account.id(), REQUESTED_ASSET_AMOUNT).unwrap();

    println!("Running SWAP tx...");
    let tx_request = TransactionRequestBuilder::swap(
        Asset::Fungible(offered_asset),
        Asset::Fungible(requested_asset),
        NoteType::Public,
    )
    .unwrap()
    .build()
    .unwrap();

    let tx = execute_tx_and_sync(&mut client1, account_a.id(), tx_request).await;

    let output_notes: Vec<OutputNote> = tx.output_notes().iter().cloned().collect();

    // TODO: Re-add future notes?
    assert_eq!(output_notes.len(), 1);

    let swap_note_tag = build_swap_tag(
        NoteType::Public,
        &Asset::Fungible(offered_asset),
        &Asset::Fungible(requested_asset),
    )
    .unwrap();

    // add swap note's tag to client2
    // we could technically avoid this step, but for the first iteration of swap notes we'll
    // require to manually add tags
    println!("Adding swap tag");
    client2.add_note_tag(swap_note_tag).await.unwrap();

    // sync on client 2, we should get the swap note
    // consume swap note with accountB, and check that the vault changed appropiately
    client2.sync_state().await.unwrap();
    println!("Consuming swap note on second client...");

    let tx_request = TransactionRequestBuilder::consume_notes(vec![output_notes[0].id()])
        .build()
        .unwrap();
    execute_tx_and_sync(&mut client2, account_b.id(), tx_request).await;

    // sync on client 1, we should get the missing payback note details.
    // try consuming the received note with accountA, it should now have 25 ETH
    client1.sync_state().await.unwrap();
    println!("Consuming swap payback note on first client...");

    let tx_request = TransactionRequestBuilder::consume_notes(vec![output_notes[0].id()])
        .build()
        .unwrap();
    execute_tx_and_sync(&mut client1, account_a.id(), tx_request).await;

    // At the end we should end up with
    //
    // - accountA: 999 BTC, 25 ETH
    // - accountB: 1 BTC, 975 ETH

    // first reload the account
    let account_a: Account = client1.get_account(account_a.id()).await.unwrap().unwrap().into();
    let account_a_assets = account_a.vault().assets();
    assert_eq!(account_a_assets.count(), 2);
    let mut account_a_assets = account_a.vault().assets();

    let asset_1 = account_a_assets.next().unwrap();
    let asset_2 = account_a_assets.next().unwrap();

    match (asset_1, asset_2) {
        (Asset::Fungible(btc_asset), Asset::Fungible(eth_asset))
            if btc_asset.faucet_id() == btc_faucet_account.id()
                && eth_asset.faucet_id() == eth_faucet_account.id() =>
        {
            assert_eq!(btc_asset.amount(), 999);
            assert_eq!(eth_asset.amount(), 25);
        },
        (Asset::Fungible(eth_asset), Asset::Fungible(btc_asset))
            if btc_asset.faucet_id() == btc_faucet_account.id()
                && eth_asset.faucet_id() == eth_faucet_account.id() =>
        {
            assert_eq!(btc_asset.amount(), 999);
            assert_eq!(eth_asset.amount(), 25);
        },
        _ => panic!("should only have fungible assets!"),
    }

    let account_b: Account = client2.get_account(account_b.id()).await.unwrap().unwrap().into();
    let account_b_assets = account_b.vault().assets();
    assert_eq!(account_b_assets.count(), 2);
    let mut account_b_assets = account_b.vault().assets();

    let asset_1 = account_b_assets.next().unwrap();
    let asset_2 = account_b_assets.next().unwrap();

    match (asset_1, asset_2) {
        (Asset::Fungible(btc_asset), Asset::Fungible(eth_asset))
            if btc_asset.faucet_id() == btc_faucet_account.id()
                && eth_asset.faucet_id() == eth_faucet_account.id() =>
        {
            assert_eq!(btc_asset.amount(), 1);
            assert_eq!(eth_asset.amount(), 975);
        },
        (Asset::Fungible(eth_asset), Asset::Fungible(btc_asset))
            if btc_asset.faucet_id() == btc_faucet_account.id()
                && eth_asset.faucet_id() == eth_faucet_account.id() =>
        {
            assert_eq!(btc_asset.amount(), 1);
            assert_eq!(eth_asset.amount(), 975);
        },
        _ => panic!("should only have fungible assets!"),
    }
}

// #[tokio::test]
// async fn test_swap_private() {
//     const OFFERED_ASSET_AMOUNT: u64 = 1;
//     const REQUESTED_ASSET_AMOUNT: u64 = 25;
//     const BTC_MINT_AMOUNT: u64 = 1000;
//     const ETH_MINT_AMOUNT: u64 = 1000;
//     let (mut client1, authenticator_1) = create_test_client().await;
//     wait_for_node(&mut client1).await;
//     let (mut client2, authenticator_2) = create_test_client().await;
//     let (mut client_with_faucets, authenticator_3) = create_test_client().await;

//     client1.sync_state().await.unwrap();
//     client2.sync_state().await.unwrap();
//     client_with_faucets.sync_state().await.unwrap();

//     // Create Client 1's basic wallet (We'll call it accountA)
//     let (account_a, ..) =
//         insert_new_wallet(&mut client1, AccountStorageMode::Private, &authenticator_1)
//             .await
//             .unwrap();

//     // Create Client 2's basic wallet (We'll call it accountB)
//     let (account_b, ..) =
//         insert_new_wallet(&mut client2, AccountStorageMode::Private, &authenticator_2)
//             .await
//             .unwrap();

//     // Create client with faucets BTC faucet (note: it's not real BTC)
//     let (btc_faucet_account, ..) = insert_new_fungible_faucet(
//         &mut client_with_faucets,
//         AccountStorageMode::Private,
//         &authenticator_3,
//     )
//     .await
//     .unwrap();
//     // Create client with faucets ETH faucet (note: it's not real ETH)
//     let (eth_faucet_account, ..) = insert_new_fungible_faucet(
//         &mut client_with_faucets,
//         AccountStorageMode::Private,
//         &authenticator_3,
//     )
//     .await
//     .unwrap();

//     // mint 1000 BTC for accountA
//     println!("minting 1000 btc for account A");
//     let account_a_mint_note_id = mint(
//         &mut client_with_faucets,
//         account_a.id(),
//         btc_faucet_account.id(),
//         NoteType::Public,
//         BTC_MINT_AMOUNT,
//     )
//     .await;
//     // mint 1000 ETH for accountB
//     println!("minting 1000 eth for account B");
//     let account_b_mint_note_id = mint(
//         &mut client_with_faucets,
//         account_b.id(),
//         eth_faucet_account.id(),
//         NoteType::Public,
//         ETH_MINT_AMOUNT,
//     )
//     .await;

//     // Sync and consume note for accountA
//     client1.sync_state().await.unwrap();
//     let client_1_consumable_notes =
//         client1.get_consumable_notes(Some(account_a.id())).await.unwrap();
//     assert!(
//         client_1_consumable_notes
//             .iter()
//             .any(|(note, _)| note.id() == account_a_mint_note_id)
//     );

//     println!("Consuming mint note on first client...");

//     let tx_request = TransactionRequestBuilder::consume_notes(vec![account_a_mint_note_id])
//         .build()
//         .unwrap();
//     execute_tx_and_sync(&mut client1, account_a.id(), tx_request).await;

//     // Sync and consume note for accountB
//     client2.sync_state().await.unwrap();
//     let client_2_consumable_notes =
//         client2.get_consumable_notes(Some(account_b.id())).await.unwrap();
//     assert!(
//         client_2_consumable_notes
//             .iter()
//             .any(|(note, _)| note.id() == account_b_mint_note_id)
//     );

//     println!("Consuming mint note on second client...");

//     let tx_request = TransactionRequestBuilder::consume_notes(vec![account_b_mint_note_id])
//         .build()
//         .unwrap();
//     execute_tx_and_sync(&mut client2, account_b.id(), tx_request).await;

//     // Create ONCHAIN swap note (clientA offers 1 BTC in exchange of 25 ETH)
//     // check that account now has 1 less BTC
//     println!("creating swap note with accountA");
//     let offered_asset = FungibleAsset::new(btc_faucet_account.id(),
// OFFERED_ASSET_AMOUNT).unwrap();     let requested_asset =
//         FungibleAsset::new(eth_faucet_account.id(), REQUESTED_ASSET_AMOUNT).unwrap();

//     println!("Running SWAP tx...");
//     let tx_request = TransactionRequestBuilder::swap(
//             Asset::Fungible(offered_asset),
//             Asset::Fungible(requested_asset),
//         NoteType::Private,
//     )
//     .unwrap()
//     .build()
//     .unwrap();

//     let tx = execute_tx_and_sync(&mut client1, account_a.id(), tx_request).await;

//     let output_notes:Vec<OutputNote> =
//         tx.output_notes().iter().cloned().collect();

//     // Export note from client 1 to client 2
//     let output_note =
//         client1.get_output_note(output_notes[0].id()).await.unwrap().unwrap();

//     let tag = build_swap_tag(
//         NoteType::Private,
//         &Asset::Fungible(offered_asset),
//         &Asset::Fungible(requested_asset),
//     )
//     .unwrap();
//     client2.add_note_tag(tag).await.unwrap();
//     client2
//         .import_note(NoteFile::NoteDetails {
//             details: output_note.try_into().unwrap(),
//             after_block_num: client1.get_sync_height().await.unwrap(),
//             tag: Some(tag),
//         })
//         .await
//         .unwrap();

//     // Sync so we get the inclusion proof info
//     client2.sync_state().await.unwrap();

//     // consume swap note with accountB, and check that the vault changed appropiately
//     println!("Consuming swap note on second client...");

//     let tx_request = TransactionRequestBuilder::consume_notes(vec![output_notes[0].id()])
//         .build()
//         .unwrap();
//     execute_tx_and_sync(&mut client2, account_b.id(), tx_request).await;

//     // sync on client 1, we should get the missing payback note details.
//     // try consuming the received note with accountA, it should now have 25 ETH
//     client1.sync_state().await.unwrap();
//     println!("Consuming swap payback note on first client...");

//     let tx_request =
//         TransactionRequestBuilder::consume_notes(vec![expected_payback_note_details[0].id()])
//             .build()
//             .unwrap();
//     execute_tx_and_sync(&mut client1, account_a.id(), tx_request).await;

//     // At the end we should end up with
//     //
//     // - accountA: 999 BTC, 25 ETH
//     // - accountB: 1 BTC, 975 ETH

//     // first reload the account
//     let account_a: Account = client1.get_account(account_a.id()).await.unwrap().unwrap().into();
//     let account_a_assets = account_a.vault().assets();
//     assert_eq!(account_a_assets.count(), 2);
//     let mut account_a_assets = account_a.vault().assets();

//     let asset_1 = account_a_assets.next().unwrap();
//     let asset_2 = account_a_assets.next().unwrap();

//     match (asset_1, asset_2) {
//         (Asset::Fungible(btc_asset), Asset::Fungible(eth_asset))
//             if btc_asset.faucet_id() == btc_faucet_account.id()
//                 && eth_asset.faucet_id() == eth_faucet_account.id() =>
//         {
//             assert_eq!(btc_asset.amount(), 999);
//             assert_eq!(eth_asset.amount(), 25);
//         },
//         (Asset::Fungible(eth_asset), Asset::Fungible(btc_asset))
//             if btc_asset.faucet_id() == btc_faucet_account.id()
//                 && eth_asset.faucet_id() == eth_faucet_account.id() =>
//         {
//             assert_eq!(btc_asset.amount(), 999);
//             assert_eq!(eth_asset.amount(), 25);
//         },
//         _ => panic!("should only have fungible assets!"),
//     }

//     let account_b: Account = client2.get_account(account_b.id()).await.unwrap().unwrap().into();
//     let account_b_assets = account_b.vault().assets();
//     assert_eq!(account_b_assets.count(), 2);
//     let mut account_b_assets = account_b.vault().assets();

//     let asset_1 = account_b_assets.next().unwrap();
//     let asset_2 = account_b_assets.next().unwrap();

//     match (asset_1, asset_2) {
//         (Asset::Fungible(btc_asset), Asset::Fungible(eth_asset))
//             if btc_asset.faucet_id() == btc_faucet_account.id()
//                 && eth_asset.faucet_id() == eth_faucet_account.id() =>
//         {
//             assert_eq!(btc_asset.amount(), 1);
//             assert_eq!(eth_asset.amount(), 975);
//         },
//         (Asset::Fungible(eth_asset), Asset::Fungible(btc_asset))
//             if btc_asset.faucet_id() == btc_faucet_account.id()
//                 && eth_asset.faucet_id() == eth_faucet_account.id() =>
//         {
//             assert_eq!(btc_asset.amount(), 1);
//             assert_eq!(eth_asset.amount(), 975);
//         },
//         _ => panic!("should only have fungible assets!"),
//     }
// }

/// Mints a note from faucet_account_id for basic_account_id with 1000 units of the corresponding
/// fungible asset, waits for inclusion and returns the note ID.
///
/// `basic_account_id` doesn't need to be tracked by the client, but `faucet_account_id` does.
async fn mint(
    client: &mut TestClient,
    basic_account_id: AccountId,
    faucet_account_id: AccountId,
    note_type: NoteType,
    mint_amount: u64,
) -> NoteId {
    // Create a Mint Tx for 1000 units of our fungible asset
    let fungible_asset = FungibleAsset::new(faucet_account_id, mint_amount).unwrap();

    println!("Minting Asset");
    let tx_request =
        TransactionRequestBuilder::mint_fungible_asset(fungible_asset, basic_account_id, note_type)
            .unwrap()
            .build()
            .unwrap();
    let tx = execute_tx_and_sync(client, faucet_account_id, tx_request.clone()).await;

    tx.output_notes().get_note(0).id()
}
