use super::Client;
use crypto::StarkField;
use miden_node_proto::{
    account_id::AccountId as ProtoAccountId, requests::SyncStateRequest,
    responses::SyncStateResponse,
};
use objects::{accounts::AccountId, notes::NoteInclusionProof, Digest};

use crate::errors::{ClientError, RpcApiError};

// CONSTANTS
// ================================================================================================

/// The number of bits to shift identifiers for in use of filters.
pub const FILTER_ID_SHIFT: u8 = 48;

impl Client {
    // SYNC STATE
    // --------------------------------------------------------------------------------------------

    /// Returns the block number of the last state sync block
    pub fn get_latest_block_number(&self) -> Result<u32, ClientError> {
        self.store
            .get_latest_block_number()
            .map_err(|err| err.into())
    }

    /// Returns the list of note tags tracked by the client.
    pub fn get_note_tags(&self) -> Result<Vec<u64>, ClientError> {
        self.store.get_note_tags().map_err(|err| err.into())
    }

    /// Adds a note tag for the client to track.
    pub fn add_note_tag(&mut self, tag: u64) -> Result<(), ClientError> {
        match self.store.add_note_tag(tag).map_err(|err| err.into()) {
            Ok(true) => Ok(()),
            Ok(false) => {
                println!("tag {} is already being tracked", tag);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// Syncs the client's state with the current state of the Miden network.
    ///
    /// Returns the block number the client has been synced to.
    pub async fn sync_state(&mut self) -> Result<u32, ClientError> {
        let block_num = self.store.get_latest_block_number()?;
        let account_ids = self.store.get_account_ids()?;
        let note_tags = self.store.get_note_tags()?;
        let nullifiers = self.store.get_unspent_input_note_nullifiers()?; // breaks

        let response = self
            .sync_state_request(block_num, &account_ids, &note_tags, &nullifiers)
            .await?;
        let incoming_block_header = response.block_header.unwrap();

        let new_block_num = incoming_block_header.block_num;
        let new_nullifiers = response
            .nullifiers
            .into_iter()
            .filter_map(|x| {
                let nullifier = x.nullifier.as_ref().unwrap().try_into().unwrap();
                if nullifiers.contains(&nullifier) {
                    Some(nullifier)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let block_header: objects::BlockHeader = incoming_block_header.try_into().unwrap();

        // Pending notes should all be `Note`s and not `RecordedNote`s
        let pending_notes = self
            .store
            .get_pending_note_hashes()
            .map_err(ClientError::StoreError)?;
        let committed_notes: Vec<(Digest, NoteInclusionProof)> = response
            .notes
            .into_iter()
            .filter_map(|note| {
                let note_hash: Digest = note.note_hash.unwrap().try_into().unwrap();
                if pending_notes.contains(&note_hash) {
                    let note_inclusion_proof = NoteInclusionProof::new(
                        block_num,
                        block_header.sub_hash(),
                        block_header.note_root(),
                        note.note_index.into(),
                        note.merkle_path.unwrap().try_into().unwrap(),
                    )
                    .unwrap();
                    Some((note_hash, note_inclusion_proof))
                } else {
                    None
                }
            })
            .collect();

        self.store
            .apply_state_sync(new_block_num, new_nullifiers, committed_notes)
            .map_err(ClientError::StoreError)?;

        Ok(response.chain_tip)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------
    /// Sends a sync state request to the Miden node and returns the response.
    async fn sync_state_request(
        &mut self,
        block_num: u32,
        account_ids: &[AccountId],
        note_tags: &[u64],
        nullifiers: &[Digest],
    ) -> Result<SyncStateResponse, ClientError> {
        let account_ids = account_ids
            .iter()
            .map(|id| ProtoAccountId { id: u64::from(*id) })
            .collect();
        let nullifiers = nullifiers
            .iter()
            .map(|nullifier| (nullifier[3].as_int() >> FILTER_ID_SHIFT) as u32)
            .collect();
        let note_tags = note_tags
            .iter()
            .map(|tag| (tag >> FILTER_ID_SHIFT) as u32)
            .collect::<Vec<_>>();

        let request = SyncStateRequest {
            block_num,
            account_ids,
            note_tags,
            nullifiers,
        };

        Ok(self
            .rpc_api
            .sync_state(request)
            .await
            .map_err(|err| ClientError::RpcApiError(RpcApiError::RequestError(err)))?
            .into_inner())
    }
}
