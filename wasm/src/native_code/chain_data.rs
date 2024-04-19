use miden_objects::crypto::rand::FeltRng;

use super::{
    rpc::NodeRpcClient, 
    Client, 
    store::Store // TODO: Add AuthInfo
};

impl<N: NodeRpcClient, R: FeltRng, S: Store> Client<N, R, S> {
    pub async fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> String { // TODO: Replace with Result<Vec<(BlockHeader, bool)>, ()>
        //self.store.get_block_headers(block_numbers).map_err(|err| ())

        "Called get_block_headers".to_string()
    }
}