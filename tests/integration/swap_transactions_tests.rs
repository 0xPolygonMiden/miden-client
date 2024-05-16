use miden_client::client::{
    accounts::{AccountStorageMode, AccountTemplate},
    transactions::transaction_request::{SwapTransactionData, TransactionTemplate},
};
use miden_objects::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset, TokenSymbol},
    notes::{NoteExecutionHint, NoteTag, NoteType},
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
    let mut client1 = create_test_client();
    wait_for_node(&mut client1).await;
    let mut client2 = create_test_client();
    let mut client_with_faucets = create_test_client();

    client1.sync_state().await.unwrap();
    client2.sync_state().await.unwrap();
    client_with_faucets.sync_state().await.unwrap();

    // Create Client 1's basic wallet (We'll call it accountA)
    let (account_a, _) = client1
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    // Create Client 2's basic wallet (We'll call it accountB)
    let (account_b, _) = client2
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    // Create client with faucets BTC faucet (note: it's not real BTC)
    let (btc_faucet_account, _) = client_with_faucets
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("BTC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();
    // Create client with faucets ETH faucet (note: it's not real ETH)
    let (eth_faucet_account, _) = client_with_faucets
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("ETH").unwrap(),
            decimals: 8,
            max_supply: 1_000_000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    // mint 1000 BTC for accountA
    println!("minting 1000 btc for account A");
    mint(
        &mut client_with_faucets,
        account_a.id(),
        btc_faucet_account.id(),
        NoteType::Public,
        BTC_MINT_AMOUNT,
    )
    .await;
    println!("minting 1000 eth for account B");
    // mint 1000 ETH for accountB
    mint(
        &mut client_with_faucets,
        account_b.id(),
        eth_faucet_account.id(),
        NoteType::Public,
        ETH_MINT_AMOUNT,
    )
    .await;

    // Sync and consume note for accountA
    client1.sync_state().await.unwrap();
    let client_1_notes = client1.get_input_notes(miden_client::store::NoteFilter::All).unwrap();
    assert_eq!(client_1_notes.len(), 1);

    println!("Consuming mint note on first client...");
    let tx_template =
        TransactionTemplate::ConsumeNotes(account_a.id(), vec![client_1_notes[0].id()]);
    let tx_request = client1.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client1, tx_request).await;

    // Sync and consume note for accountB
    client2.sync_state().await.unwrap();
    let client_2_notes = client2.get_input_notes(miden_client::store::NoteFilter::All).unwrap();
    assert_eq!(client_2_notes.len(), 1);

    println!("Consuming mint note on second client...");
    let tx_template =
        TransactionTemplate::ConsumeNotes(account_b.id(), vec![client_2_notes[0].id()]);
    let tx_request = client2.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client2, tx_request).await;

    // Create ONCHAIN swap note (clientA offers 1 BTC in exchange of 25 ETH)
    // check that account now has 1 less BTC
    println!("creating swap note with accountA");
    let offered_asset = FungibleAsset::new(btc_faucet_account.id(), OFFERED_ASSET_AMOUNT).unwrap();
    let requested_asset =
        FungibleAsset::new(eth_faucet_account.id(), REQUESTED_ASSET_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::Swap(
        SwapTransactionData::new(
            account_a.id(),
            Asset::Fungible(offered_asset),
            Asset::Fungible(requested_asset),
        ),
        NoteType::Public,
    );
    println!("Running SWAP tx...");
    let tx_request = client1.build_transaction_request(tx_template).unwrap();

    let expected_output_notes = tx_request.expected_output_notes().to_vec();
    let expected_payback_note_details = tx_request.expected_partial_notes().to_vec();
    assert_eq!(expected_output_notes.len(), 1);
    assert_eq!(expected_payback_note_details.len(), 1);

    execute_tx_and_sync(&mut client1, tx_request).await;

    let payback_note_tag =
        build_swap_tag(NoteType::Public, btc_faucet_account.id(), eth_faucet_account.id());

    // add swap note's tag to both client 1 and client 2 (TODO: check if it's needed for both)
    // we could technically avoid this step, but for the first iteration of swap notes we'll
    // require to manually add tags
    println!("Adding swap tags");
    client1.add_note_tag(payback_note_tag).unwrap();
    client2.add_note_tag(payback_note_tag).unwrap();

    // sync on client 2, we should get the swap note
    // consume swap note with accountB, and check that the vault changed appropiately
    client2.sync_state().await.unwrap();
    println!("Consuming swap note on second client...");
    let tx_template =
        TransactionTemplate::ConsumeNotes(account_b.id(), vec![expected_output_notes[0].id()]);
    let tx_request = client2.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client2, tx_request).await;

    // sync on client 1, we should get the missing payback note details.
    // try consuming the received note with accountA, it should now have 25 ETH
    client1.sync_state().await.unwrap();
    println!("Consuming swap payback note on first client...");
    let tx_template = TransactionTemplate::ConsumeNotes(
        account_a.id(),
        vec![expected_payback_note_details[0].id()],
    );
    let tx_request = client1.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client1, tx_request).await;

    // At the end we should end up with
    //
    // - accountA: 999 BTC, 25 ETH
    // - accountB: 1 BTC, 975 ETH

    // first reload the account
    let (account_a, _) = client1.get_account(account_a.id()).unwrap();
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

    let (account_b, _) = client2.get_account(account_b.id()).unwrap();
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

#[tokio::test]
async fn test_swap_offchain() {
    const OFFERED_ASSET_AMOUNT: u64 = 1;
    const REQUESTED_ASSET_AMOUNT: u64 = 25;
    const BTC_MINT_AMOUNT: u64 = 1000;
    const ETH_MINT_AMOUNT: u64 = 1000;
    let mut client1 = create_test_client();
    wait_for_node(&mut client1).await;
    let mut client2 = create_test_client();
    let mut client_with_faucets = create_test_client();

    client1.sync_state().await.unwrap();
    client2.sync_state().await.unwrap();
    client_with_faucets.sync_state().await.unwrap();

    // Create Client 1's basic wallet (We'll call it accountA)
    let (account_a, _) = client1
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    // Create Client 2's basic wallet (We'll call it accountB)
    let (account_b, _) = client2
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    // Create client with faucets BTC faucet (note: it's not real BTC)
    let (btc_faucet_account, _) = client_with_faucets
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("BTC").unwrap(),
            decimals: 8,
            max_supply: 1_000_000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();
    // Create client with faucets ETH faucet (note: it's not real ETH)
    let (eth_faucet_account, _) = client_with_faucets
        .new_account(AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new("ETH").unwrap(),
            decimals: 8,
            max_supply: 1_000_000,
            storage_mode: AccountStorageMode::Local,
        })
        .unwrap();

    // mint 1000 BTC for accountA
    println!("minting 1000 btc for account A");
    mint(
        &mut client_with_faucets,
        account_a.id(),
        btc_faucet_account.id(),
        NoteType::Public,
        BTC_MINT_AMOUNT,
    )
    .await;
    // mint 1000 ETH for accountB
    println!("minting 1000 eth for account B");
    mint(
        &mut client_with_faucets,
        account_b.id(),
        eth_faucet_account.id(),
        NoteType::Public,
        ETH_MINT_AMOUNT,
    )
    .await;

    // Sync and consume note for accountA
    client1.sync_state().await.unwrap();
    let client_1_notes = client1.get_input_notes(miden_client::store::NoteFilter::All).unwrap();
    assert_eq!(client_1_notes.len(), 1);

    println!("Consuming mint note on first client...");
    let tx_template =
        TransactionTemplate::ConsumeNotes(account_a.id(), vec![client_1_notes[0].id()]);
    let tx_request = client1.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client1, tx_request).await;

    // Sync and consume note for accountB
    client2.sync_state().await.unwrap();
    let client_2_notes = client2.get_input_notes(miden_client::store::NoteFilter::All).unwrap();
    assert_eq!(client_2_notes.len(), 1);

    println!("Consuming mint note on second client...");
    let tx_template =
        TransactionTemplate::ConsumeNotes(account_b.id(), vec![client_2_notes[0].id()]);
    let tx_request = client2.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client2, tx_request).await;

    // Create ONCHAIN swap note (clientA offers 1 BTC in exchange of 25 ETH)
    // check that account now has 1 less BTC
    println!("creating swap note with accountA");
    let offered_asset = FungibleAsset::new(btc_faucet_account.id(), OFFERED_ASSET_AMOUNT).unwrap();
    let requested_asset =
        FungibleAsset::new(eth_faucet_account.id(), REQUESTED_ASSET_AMOUNT).unwrap();
    let tx_template = TransactionTemplate::Swap(
        SwapTransactionData::new(
            account_a.id(),
            Asset::Fungible(offered_asset),
            Asset::Fungible(requested_asset),
        ),
        NoteType::OffChain,
    );
    println!("Running SWAP tx...");
    let tx_request = client1.build_transaction_request(tx_template).unwrap();

    let expected_output_notes = tx_request.expected_output_notes().to_vec();
    let expected_payback_note_details = tx_request.expected_partial_notes().to_vec();
    assert_eq!(expected_output_notes.len(), 1);
    assert_eq!(expected_payback_note_details.len(), 1);

    execute_tx_and_sync(&mut client1, tx_request).await;

    // Export note from client 1 to client 2
    let exported_note = client1.get_output_note(expected_output_notes[0].id()).unwrap();

    client2
        .import_input_note(exported_note.try_into().unwrap(), true)
        .await
        .unwrap();

    // Sync so we get the inclusion proof info
    client2.sync_state().await.unwrap();

    // consume swap note with accountB, and check that the vault changed appropiately
    println!("Consuming swap note on second client...");
    let tx_template =
        TransactionTemplate::ConsumeNotes(account_b.id(), vec![expected_output_notes[0].id()]);
    let tx_request = client2.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client2, tx_request).await;

    // sync on client 1, we should get the missing payback note details.
    // try consuming the received note with accountA, it should now have 25 ETH
    client1.sync_state().await.unwrap();
    println!("Consuming swap payback note on first client...");
    let tx_template = TransactionTemplate::ConsumeNotes(
        account_a.id(),
        vec![expected_payback_note_details[0].id()],
    );
    let tx_request = client1.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(&mut client1, tx_request).await;

    // At the end we should end up with
    //
    // - accountA: 999 BTC, 25 ETH
    // - accountB: 1 BTC, 975 ETH

    // first reload the account
    let (account_a, _) = client1.get_account(account_a.id()).unwrap();
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

    let (account_b, _) = client2.get_account(account_b.id()).unwrap();
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

/// Returns a note tag for a swap note with the specified parameters.
///
/// Use case ID for the returned tag is set to 0.
///
/// Tag payload is constructed by taking asset tags (8 bits of faucet ID) and concatenating them
/// together as offered_asset_tag + requested_asset tag.
///
/// Network execution hint for the returned tag is set to `Local`.
///
/// Based on miden-base's implementation (<https://github.com/0xPolygonMiden/miden-base/blob/9e4de88031b55bcc3524cb0ccfb269821d97fb29/miden-lib/src/notes/mod.rs#L153>)
fn build_swap_tag(
    note_type: NoteType,
    offered_asset_faucet_id: AccountId,
    requested_asset_faucet_id: AccountId,
) -> NoteTag {
    const SWAP_USE_CASE_ID: u16 = 0;

    // get bits 4..12 from faucet IDs of both assets, these bits will form the tag payload; the
    // reason we skip the 4 most significant bits is that these encode metadata of underlying
    // faucets and are likely to be the same for many different faucets.

    let offered_asset_id: u64 = offered_asset_faucet_id.into();
    let offered_asset_tag = (offered_asset_id >> 52) as u8;

    let requested_asset_id: u64 = requested_asset_faucet_id.into();
    let requested_asset_tag = (requested_asset_id >> 52) as u8;

    let payload = ((offered_asset_tag as u16) << 8) | (requested_asset_tag as u16);

    let execution = NoteExecutionHint::Local;
    match note_type {
        NoteType::Public => NoteTag::for_public_use_case(SWAP_USE_CASE_ID, payload, execution),
        _ => NoteTag::for_local_use_case(SWAP_USE_CASE_ID, payload),
    }
    .unwrap()
}

/// Mints a note from faucet_account_id for basic_account_id, waits for inclusion and returns it
/// with 1000 units of the corresponding fungible asset
///
/// `basic_account_id` does not need to be tracked by the client, but `faucet_account_id` does
async fn mint(
    client: &mut TestClient,
    basic_account_id: AccountId,
    faucet_account_id: AccountId,
    note_type: NoteType,
    mint_amount: u64,
) {
    // Create a Mint Tx for 1000 units of our fungible asset
    let fungible_asset = FungibleAsset::new(faucet_account_id, mint_amount).unwrap();
    let tx_template =
        TransactionTemplate::MintFungibleAsset(fungible_asset, basic_account_id, note_type);

    println!("Minting Asset");
    let tx_request = client.build_transaction_request(tx_template).unwrap();
    execute_tx_and_sync(client, tx_request.clone()).await;
}
