

use crypto::{
    merkle::{MerklePath, PartialMmr},
    utils::Serializable,
};
use miden_node_proto::{mmr::MmrDelta, responses::AccountHashUpdate};

use objects::{
    accounts::AccountStub,
    notes::{NoteId, NoteInclusionProof},
    BlockHeader, Digest,
};
use rusqlite::params;

use crate::{
    errors::StoreError,
    store::{
        transactions::TransactionFilter,
    },
};

use super::Store;

impl Store {
    // STATE SYNC
    // --------------------------------------------------------------------------------------------

    /// Returns the note tags that the client is interested in.
    pub fn get_note_tags(&self) -> Result<Vec<u64>, StoreError> {
        const QUERY: &str = "SELECT tags FROM state_sync";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .and_then(|v: String| {
                        serde_json::from_str(&v).map_err(StoreError::JsonDataDeserializationError)
                    })
            })
            .next()
            .expect("state sync tags exist")
    }

    /// Adds a note tag to the list of tags that the client is interested in.
    pub fn add_note_tag(&mut self, tag: u64) -> Result<bool, StoreError> {
        let mut tags = self.get_note_tags()?;
        if tags.contains(&tag) {
            return Ok(false);
        }
        tags.push(tag);
        let tags = serde_json::to_string(&tags).map_err(StoreError::InputSerializationError)?;

        const QUERY: &str = "UPDATE state_sync SET tags = ?";
        self.db
            .execute(QUERY, params![tags])
            .map_err(StoreError::QueryError)
            .map(|_| ())?;

        Ok(true)
    }

    /// Returns the block number of the last state sync block
    pub fn get_sync_height(&self) -> Result<u32, StoreError> {
        const QUERY: &str = "SELECT block_num FROM state_sync";

        self.db
            .prepare(QUERY)
            .map_err(StoreError::QueryError)?
            .query_map([], |row| row.get(0))
            .expect("no binding parameters used in query")
            .map(|result| {
                result
                    .map_err(StoreError::ColumnParsingError)
                    .map(|v: i64| v as u32)
            })
            .next()
            .expect("state sync block number exists")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_state_sync(
        &mut self,
        current_block_num: u32,
        block_header: BlockHeader,
        requested_header_block_path: MerklePath,
        nullifiers: Vec<Digest>,
        account_updates: Vec<AccountHashUpdate>,
        mmr_delta: Option<MmrDelta>,
        committed_notes: Vec<(Digest, NoteInclusionProof)>,
    ) -> Result<(), StoreError> {
        // get current nodes on table
        // we need to do this here because creating a sql tx borrows a mut reference
        let (current_block_header, header_had_notes) =
            self.get_block_header_by_num(current_block_num)?;

        let current_peaks = self.get_chain_mmr_peaks_by_block_num(current_block_num)?;

        let uncommitted_transactions = self.get_transactions(TransactionFilter::Uncomitted)?;

        let current_accounts: Vec<AccountStub> = self
            .get_accounts()?
            .iter()
            .map(|(acc, _)| acc.clone())
            .collect();

        let tx = self
            .db
            .transaction()
            .map_err(StoreError::TransactionError)?;

        // Check if the returned account hashes match latest account hashes in the database
        Self::check_account_hashes(&account_updates, &current_accounts)?;

        // update state sync block number
        const BLOCK_NUMBER_QUERY: &str = "UPDATE state_sync SET block_num = ?";
        tx.execute(BLOCK_NUMBER_QUERY, params![block_header.block_num()])
            .map_err(StoreError::QueryError)?;

        // update spent notes
        for nullifier in nullifiers {
            const SPENT_QUERY: &str =
                "UPDATE input_notes SET status = 'consumed' WHERE nullifier = ?";
            let nullifier = nullifier.to_string();
            tx.execute(SPENT_QUERY, params![nullifier])
                .map_err(StoreError::QueryError)?;
        }

        // build partial mmr from the nodes - partial_mmr should be on memory as part of our store
        let mut partial_mmr: PartialMmr = PartialMmr::from_peaks(current_peaks);

        if let Some(mmr_delta) = mmr_delta {
            // apply the delta
            let mmr_delta: crypto::merkle::MmrDelta = mmr_delta
                .try_into()
                .map_err(StoreError::RpcTypeConversionFailure)?;

            let new_authentication_nodes =
                partial_mmr.apply(mmr_delta).map_err(StoreError::MmrError)?;

            if header_had_notes {
                partial_mmr
                    .add(
                        current_block_num as usize,
                        current_block_header.hash(),
                        &requested_header_block_path,
                    )
                    .map_err(StoreError::MmrError)?;
            }

            Store::insert_chain_mmr_nodes(&tx, new_authentication_nodes)?;
        }

        let header_has_interesting_notes = !committed_notes.is_empty();

        Store::insert_block_header(
            &tx,
            block_header,
            partial_mmr.peaks(),
            header_has_interesting_notes,
        )?;

        // update tracked notes
        for (note_id, inclusion_proof) in committed_notes.iter() {
            const SPENT_QUERY: &str =
                "UPDATE input_notes SET status = 'committed', inclusion_proof = ? WHERE note_id = ?";

            let inclusion_proof = Some(inclusion_proof.to_bytes());
            tx.execute(SPENT_QUERY, params![inclusion_proof, note_id.to_string()])
                .map_err(StoreError::QueryError)?;
        }

        let note_ids: Vec<NoteId> = committed_notes
            .iter()
            .map(|(id, _)| NoteId::from(*id))
            .collect();

        Store::mark_transactions_as_committed_by_note_id(
            &uncommitted_transactions,
            &note_ids,
            block_header.block_num(),
            &tx,
        )?;

        // commit the updates
        tx.commit().map_err(StoreError::QueryError)?;

        Ok(())
    }

    fn check_account_hashes(
        account_updates: &[AccountHashUpdate],
        current_accounts: &[AccountStub],
    ) -> Result<(), StoreError> {
        for account_update in account_updates {
            if let (Some(update_account_id), Some(remote_account_hash)) =
                (&account_update.account_id, &account_update.account_hash)
            {
                let update_account_id: u64 = update_account_id.clone().into();
                if let Some(acc_stub) = current_accounts
                    .iter()
                    .find(|acc| update_account_id == u64::from(acc.id()))
                {
                    let remote_account_hash: Digest = remote_account_hash
                        .try_into()
                        .map_err(StoreError::RpcTypeConversionFailure)?;

                    if remote_account_hash != acc_stub.hash() {
                        return Err(StoreError::AccountHashMismatch(
                            update_account_id
                                .try_into()
                                .map_err(StoreError::AccountError)?,
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
