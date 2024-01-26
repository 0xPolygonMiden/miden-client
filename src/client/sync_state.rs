use super::Client;
use crypto::{merkle::MmrPeaks, StarkField};
use miden_node_proto::{
    account::AccountId as ProtoAccountId,
    note::NoteSyncRecord,
    requests::{GetBlockHeaderByNumberRequest, SyncStateRequest},
    responses::SyncStateResponse,
};

use objects::{accounts::AccountId, notes::NoteInclusionProof, BlockHeader, Digest};

use crate::{
    errors::{ClientError, StoreError},
    store::Store,
};

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

    /// Returns the block number of the last state sync block.
    pub fn get_sync_height(&self) -> Result<u32, ClientError> {
        self.store.get_sync_height().map_err(|err| err.into())
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
    /// Before doing so, it ensures the genesis block exists in the local store.
    ///
    /// Returns the block number the client has been synced to.
    pub async fn sync_state(&mut self) -> Result<u32, ClientError> {
        self.ensure_genesis_in_place().await?;
        loop {
            let response = self.single_sync_state().await?;
            if let SyncStatus::SyncedToLastBlock(v) = response {
                return Ok(v);
            }
        }
    }

    /// Attempts to retrieve the genesis block from the store. If not found,
    /// it requests it from the node and store it
    async fn ensure_genesis_in_place(&mut self) -> Result<(), ClientError> {
        let genesis = self.store.get_block_header_by_num(0);

        match genesis {
            Ok(_) => Ok(()),
            Err(StoreError::BlockHeaderNotFound(0)) => self.retrieve_and_store_genesis().await,
            Err(err) => Err(ClientError::StoreError(err)),
        }
    }

    /// Calls `get_block_header_by_number` requesting the genesis block and storing it
    /// in the local database
    async fn retrieve_and_store_genesis(&mut self) -> Result<(), ClientError> {
        let genesis_block = self
            .rpc_api
            .get_block_header_by_number(GetBlockHeaderByNumberRequest { block_num: Some(0) })
            .await
            .map_err(ClientError::RpcApiError)?
            .into_inner();

        let genesis_block: objects::BlockHeader = genesis_block
            .block_header
            .ok_or(ClientError::RpcExpectedFieldMissing(
                "Expected block header in genesis block request".to_string(),
            ))?
            .try_into()?;

        let tx = self.store.db.transaction()?;

        Store::insert_block_header(
            &tx,
            genesis_block,
            MmrPeaks::new(0, vec![]).expect("Blank MmrPeaks"),
            false,
        )?;

        tx.commit()?;
        Ok(())
    }

    async fn single_sync_state(&mut self) -> Result<SyncStatus, ClientError> {
        let current_block_num = self.store.get_sync_height()?;
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

        let incoming_block_header =
            response
                .block_header
                .as_ref()
                .ok_or(ClientError::RpcExpectedFieldMissing(format!(
                    "Expected block header for response: {:?}",
                    &response
                )))?;
        let incoming_block_header: BlockHeader = incoming_block_header.try_into()?;

        if incoming_block_header.block_num() == current_block_num {
            return Ok(SyncStatus::SyncedToLastBlock(current_block_num));
        }

        let response_nullifiers = response
            .nullifiers
            .clone()
            .into_iter()
            .map(|x| {
                x.nullifier
                    .ok_or(ClientError::RpcExpectedFieldMissing(format!(
                        "Expected nullifier for response {:?}",
                        &response.clone()
                    )))
            })
            .collect::<Result<Vec<_>, ClientError>>()?;

        let parsed_new_nullifiers = response_nullifiers
            .into_iter()
            .map(|response_nullifier| {
                response_nullifier
                    .try_into()
                    .map_err(ClientError::RpcTypeConversionFailure)
            })
            .collect::<Result<Vec<_>, ClientError>>()?;

        let new_nullifiers = parsed_new_nullifiers
            .into_iter()
            .filter(|nullifier| nullifiers.contains(nullifier))
            .collect();

        let committed_notes =
            self.get_newly_committed_note_info(&response.notes, &incoming_block_header)?;

        let mmr_delta = response
            .mmr_delta
            .ok_or(ClientError::RpcExpectedFieldMissing(
                "MmrDelta missing on node's response".to_string(),
            ))?;

        self.store
            .apply_sync_state(
                current_block_num,
                incoming_block_header,
                new_nullifiers,
                response.accounts,
                mmr_delta,
                committed_notes,
            )?;

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
            .get_input_notes(crate::store::notes::InputNoteFilter::Pending)?
            .iter()
            .map(|n| n.note().id().inner())
            .collect();

        let notes_with_hashes_and_merkle_paths =
            notes
                .iter()
                .map(|note_record| {
                    // Handle Options first
                    let note_hash = note_record.note_hash.clone().ok_or(
                        ClientError::RpcExpectedFieldMissing(format!(
                            "Expected note hash for response note record {:?}",
                            &note_record
                        )),
                    )?;
                    let note_merkle_path = note_record.merkle_path.clone().ok_or(
                        ClientError::RpcExpectedFieldMissing(format!(
                            "Expected merkle path for response note record {:?}",
                            &note_record
                        )),
                    )?;
                    // Handle casting after
                    let note_hash = note_hash.try_into()?;
                    let merkle_path: crypto::merkle::MerklePath = note_merkle_path.try_into()?;

                    Ok((note_record, note_hash, merkle_path))
                })
                .collect::<Result<Vec<_>, ClientError>>()?;

        notes_with_hashes_and_merkle_paths
            .iter()
            .filter_map(|(note, note_id, merkle_path)| {
                if pending_notes.contains(note_id) {
                    // FIXME: This removal is to accomodate a problem with how the node constructs paths where
                    // they are constructed using note ID instead of authentication hash, so for now we remove the first
                    // node here.
                    //
                    // See: https://github.com/0xPolygonMiden/miden-node/blob/main/store/src/state.rs#L274
                    let mut merkle_path = merkle_path.clone();
                    if merkle_path.len() > 0 {
                        let _ = merkle_path.remove(0);
                    }
                    let note_id_and_proof = NoteInclusionProof::new(
                        block_header.block_num(),
                        block_header.sub_hash(),
                        block_header.note_root(),
                        note.note_index.into(),
                        merkle_path,
                    )
                    .map_err(ClientError::NoteError)
                    .map(|proof| (*note_id, proof));

                    Some(note_id_and_proof)
                } else {
                    None
                }
            })
            .collect()
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

        Ok(self.rpc_api.sync_state(request).await?.into_inner())
    }
}
