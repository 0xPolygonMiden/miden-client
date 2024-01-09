use super::Client;

#[cfg(test)]
use crate::errors::ClientError;
#[cfg(test)]
use objects::BlockHeader;
#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use crypto::merkle::InOrderIndex;
#[cfg(test)]
use objects::Digest;

impl Client {
    #[cfg(test)]
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

    #[cfg(test)]
    pub fn get_chain_mmr_nodes(
        &mut self,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, ClientError> {
        let chain_mmr_nodes = self.store.get_full_chain_mmr_nodes()
            .map_err(ClientError::StoreError)?;


        Ok(chain_mmr_nodes)
    }
}
