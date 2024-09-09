use clap::Parser;
use miden_client::{
    auth::TransactionAuthenticator, crypto::FeltRng, rpc::NodeRpcClient, store::Store,
    transactions::TransactionProver, Client,
};

#[derive(Debug, Parser, Clone)]
#[clap(about = "Sync this client with the latest state of the Miden network.")]
pub struct SyncCmd {
    /// If enabled, the ignored notes will also be updated by fetching them directly from the node.
    #[clap(short, long)]
    update_ignored: bool,
}

impl SyncCmd {
    pub async fn execute<
        N: NodeRpcClient,
        R: FeltRng,
        S: Store,
        A: TransactionAuthenticator,
        P: TransactionProver,
    >(
        &self,
        mut client: Client<N, R, S, A, P>,
    ) -> Result<(), String> {
        let mut new_details = client.sync_state().await?;
        if self.update_ignored {
            new_details.combine_with(&client.update_ignored_notes().await?);
        }

        println!("State synced to block {}", new_details.block_num);
        println!("New public notes: {}", new_details.new_notes);
        println!("Tracked notes updated: {}", new_details.new_inclusion_proofs);
        println!("Tracked notes consumed: {}", new_details.new_nullifiers);
        println!("Tracked accounts updated: {}", new_details.updated_onchain_accounts);
        println!("Commited transactions: {}", new_details.commited_transactions);
        Ok(())
    }
}
