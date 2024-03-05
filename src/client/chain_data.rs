use super::{rpc::NodeRpcClient, Client, Store};
use miden_tx::DataStore;

#[cfg(test)]
use crate::errors::ClientError;
#[cfg(test)]
use objects::BlockHeader;

impl<N: NodeRpcClient, S: Store, D: DataStore> Client<N, S, D> {
    #[cfg(test)]
    pub fn get_block_headers_in_range(
        &self,
        start: u32,
        finish: u32,
    ) -> Result<Vec<(BlockHeader, bool)>, ClientError> {
        self.store
            .get_block_headers(&(start..=finish).collect::<Vec<u32>>())
            .map_err(ClientError::StoreError)
    }

    #[cfg(test)]
    pub fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, ClientError> {
        self.store
            .get_block_headers(block_numbers)
            .map_err(ClientError::StoreError)
    }
}
