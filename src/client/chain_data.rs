use crate::{
    client::{rpc::NodeRpcClient, Client},
    errors::ClientError,
    store::Store,
};
use miden_objects::BlockHeader;
use miden_tx::DataStore;

impl<N: NodeRpcClient, S: Store, D: DataStore> Client<N, S, D> {
    pub fn get_block_headers_in_range(
        &self,
        start: u32,
        finish: u32,
    ) -> Result<Vec<(BlockHeader, bool)>, ClientError> {
        self.store
            .get_block_headers(&(start..=finish).collect::<Vec<u32>>())
            .map_err(ClientError::StoreError)
    }

    pub fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, ClientError> {
        self.store
            .get_block_headers(block_numbers)
            .map_err(ClientError::StoreError)
    }
}
