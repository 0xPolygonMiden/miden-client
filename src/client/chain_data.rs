use miden_objects::crypto::rand::FeltRng;
#[cfg(test)]
use miden_objects::BlockHeader;
use miden_tx::auth::TransactionAuthenticator;

#[cfg(test)]
use crate::{
    client::{rpc::NodeRpcClient, Client},
    errors::ClientError,
    store::Store,
};

#[cfg(test)]
impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
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
