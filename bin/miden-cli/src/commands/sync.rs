use clap::Parser;
use miden_client::{
    auth::TransactionAuthenticator, crypto::FeltRng, rpc::NodeRpcClient, store::Store, Client,
};

#[derive(Debug, Parser, Clone)]
#[clap(about = "Sync this client with the latest state of the Miden network.")]
pub struct SyncCmd {
    /// If enabled, the ignored notes will also be updated by fetching them directly from the node.
    #[clap(short, long)]
    update_ignored: bool,
}

impl SyncCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        let mut new_details = client.sync_state().await?;
        if self.update_ignored {
            new_details.combine_with(client.update_ignored_notes().await?);
        }

        println!("State synced to block {}", new_details.block_num);
        println!("New public notes: {}", new_details.received_notes.len());
        println!("Tracked notes updated: {}", new_details.committed_notes.len());
        println!("Tracked notes consumed: {}", new_details.consumed_notes.len());
        println!("Tracked accounts updated: {}", new_details.updated_accounts.len());
        println!("Commited transactions: {}", new_details.committed_transactions.len());
        Ok(())
    }
}
