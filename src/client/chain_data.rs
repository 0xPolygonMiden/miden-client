use super::Client;

use crate::errors::ClientError;
use objects::BlockHeader;

impl Client {
    pub fn get_block_headers(
        &self,
        start: u32,
        finish: u32,
    ) -> Result<Vec<BlockHeader>, ClientError> {
        let mut headers = Vec::new();
        for block_number in start..=finish {
            match self.store.get_block_header_by_num(block_number) {
                Ok(block_header) => headers.push(block_header),
                Err(_) => {} // ignore missing block headers
            }
        }

        Ok(headers)
    }
}
