use super::Client;
use crypto::StarkField;
use miden_node_proto::{
    account::AccountId as ProtoAccountId, note::NoteSyncRecord, requests::SyncStateRequest,
    responses::SyncStateResponse,
};

use objects::{accounts::AccountId, notes::NoteInclusionProof, BlockHeader, Digest};

use crate::errors::{ClientError, RpcApiError};

pub enum SyncStatus {
    SyncedToLastBlock(u32),
    SyncedToBlock(u32),
}

// CONSTANTS
// ================================================================================================

/// The number of bits to shift identifiers for in use of filters.
pub const FILTER_ID_SHIFT: u8 = 48;

impl Client {
    // SYNC STATE
    // --------------------------------------------------------------------------------------------

    /// Returns the block number of the last state sync block
    pub fn get_latest_block_num(&self) -> Result<u32, ClientError> {
        self.store.get_latest_block_num().map_err(|err| err.into())
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
        loop {
            let response = self.single_sync_state().await?;
            if let SyncStatus::SyncedToLastBlock(v) = response {
                return Ok(v);
            }
        }
    }

    async fn single_sync_state(&mut self) -> Result<SyncStatus, ClientError> {
        let current_block_num = self.store.get_latest_block_num()?;
        let account_ids = self.store.get_account_ids()?;
        let note_tags: Vec<u64> = self
            .store
            .get_accounts()
            .unwrap()
            .into_iter()
            .map(|(a, _s)| a.id().into())
            .collect();

        let nullifiers = self.store.get_unspent_input_note_nullifiers()?;
        let response = self
            .sync_state_request(current_block_num, &account_ids, &note_tags, &nullifiers)
            .await?;

        let incoming_block_header = response.block_header.unwrap();
        let incoming_block_header: BlockHeader = incoming_block_header
            .try_into()
            .map_err(ClientError::RpcTypeConversionFailure)?;

        if incoming_block_header.block_num() == current_block_num
            && (current_block_num != 0 || self.store.get_block_header_by_num(0).is_ok())
        {
            return Ok(SyncStatus::SyncedToLastBlock(current_block_num));
        }

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

        let committed_notes =
            self.get_newly_committed_note_info(&response.notes, &incoming_block_header)?;

        self.store
            .apply_state_sync(
                current_block_num,
                incoming_block_header,
                new_nullifiers,
                response.accounts,
                response.mmr_delta,
                committed_notes,
            )
            .map_err(ClientError::StoreError)?;

        if response.chain_tip == incoming_block_header.block_num() {
            Ok(SyncStatus::SyncedToLastBlock(response.chain_tip))
        } else {
            Ok(SyncStatus::SyncedToBlock(incoming_block_header.block_num()))
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Extracts information about notes that the client is interested in, creating the note inclusion
    /// proof in order to correctly update store data
    fn get_newly_committed_note_info(
        &self,
        notes: &[NoteSyncRecord],
        block_header: &BlockHeader,
    ) -> Result<Vec<(Digest, NoteInclusionProof)>, ClientError> {
        let pending_notes: Vec<Digest> = self
            .store
            .get_input_notes(crate::store::notes::InputNoteFilter::Pending)
            .map_err(ClientError::StoreError)?
            .iter()
            .map(|n| n.note().id().inner())
            .collect();

        Ok(notes
            .iter()
            .filter_map(|note| {
                let note_hash = note.note_hash.clone().unwrap().try_into().unwrap();
                if pending_notes.contains(&note_hash) {
                    let note_inclusion_proof = NoteInclusionProof::new(
                        block_header.block_num(),
                        block_header.sub_hash(),
                        block_header.note_root(),
                        note.note_index.into(),
                        note.merkle_path.clone().unwrap().try_into().unwrap(),
                    )
                    .unwrap();

                    Some((note_hash, note_inclusion_proof))
                } else {
                    None
                }
            })
            .collect())
    }

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
