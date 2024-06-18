use miden_client::{rpc::NodeRpcClient, store::Store, Client};
use miden_objects::crypto::rand::FeltRng;
use miden_tx::auth::TransactionAuthenticator;

pub async fn sync_state<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
    mut client: Client<N, R, S, A>,
) -> Result<(), String> {
    let new_details = client.sync_state().await?;
    println!("State synced to block {}", new_details.block_num);
    println!("New public notes: {}", new_details.new_notes);
    println!("Tracked notes updated: {}", new_details.new_inclusion_proofs);
    println!("Tracked notes consumed: {}", new_details.new_nullifiers);
    println!("Tracked accounts updated: {}", new_details.updated_onchain_accounts);
    println!("Commited transactions: {}", new_details.commited_transactions);
    Ok(())
}
