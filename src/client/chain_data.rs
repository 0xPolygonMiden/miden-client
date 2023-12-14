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
            if let Ok(block_header) = self.store.get_block_header_by_num(block_number) {
                headers.push(block_header)
            }
        }

        Ok(headers)
    }
}
