use super::Client;

#[cfg(test)]
use crate::{errors::ClientError, store::chain_data::BlockFilter};
#[cfg(test)]
use objects::BlockHeader;

impl Client {
    #[cfg(test)]
    pub fn get_block_headers_in_range(
        &self,
        start: u32,
        finish: u32,
    ) -> Result<Vec<BlockHeader>, ClientError> {
        self.store.get_block_headers(BlockFilter::Range(start, finish))
            .map_err(ClientError::StoreError)
    }

    #[cfg(test)]
    pub fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<BlockHeader>, ClientError> {
        self.store.get_block_headers(BlockFilter::List(block_numbers))
            .map_err(ClientError::StoreError)
    }
}
