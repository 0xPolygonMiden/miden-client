use super::common::*;

// SWAP FULLY ONCHAIN
// ================================================================================================

#[tokio::test]
async fn test_swap_fully_onchain() {
    let mut client1 = create_test_client();
    wait_for_node(&mut client1).await;
    let mut client2 = create_test_client();
    let mut client_with_faucets = create_test_client();

    client1.sync_state().await.unwrap();
    client2.sync_state().await.unwrap();
    client_with_faucets.sync_state().await.unwrap();

    // Create Client 1's basic wallet (We'll call it accountA)

    // Create Client 2's basic wallet (We'll call it accountB)

    // Create client with faucets BTC faucet (note: it's not real BTC)
    // Create client with faucets ETH faucet (note: it's not real ETH)

    // mint 1000 BTC for accountA
    // mint 1000 ETH for accountB

    // Sync and consume note for accountA
    // Sync and consume note for accountB

    // Create ONCHAIN swap note (clientA offers 1 BTC in exchange of 25 ETH)
    // check that account now has 1 less BTC

    // add swap note's tag to both client 1 and client 2 (TODO: check if it's needed for both)
    // we could technically avoid this step, but for the first iteration of swap notes we'll
    // require to manually add tags

    // sync on client 2, we should get the swap note
    // consume swap note with accountB, and check that the vault changed appropiately

    // sync on client 1, we should get the missing payback note details.
    // try consuming the received note with accountA, it should now have 25 ETH
}
