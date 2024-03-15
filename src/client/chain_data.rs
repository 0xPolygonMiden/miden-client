#[cfg(test)]
use miden_objects::BlockHeader;

#[cfg(test)]
use crate::{
    client::{rpc::NodeRpcClient, Client},
    errors::ClientError,
    store::Store,
};

#[cfg(test)]
impl<N: NodeRpcClient, S: Store> Client<N, S> {
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
        self.store.get_block_headers(block_numbers).map_err(ClientError::StoreError)
    }
}
