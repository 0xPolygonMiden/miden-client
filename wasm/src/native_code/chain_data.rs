use miden_objects::{crypto::rand::FeltRng, BlockHeader};

use super::{
    errors::ClientError, rpc::NodeRpcClient, store::Store, Client // TODO: Add AuthInfo
};

impl<N: NodeRpcClient, R: FeltRng, S: Store> Client<N, R, S> {
    pub async fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, ClientError> {
        self.store.get_block_headers(block_numbers).await.map_err(ClientError::StoreError)
    }
}