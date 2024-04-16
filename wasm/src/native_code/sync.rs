use super::{
    rpc::NodeRpcClient, 
    Client, 
    store::Store // TODO: Add AuthInfo
};

pub enum SyncStatus {
    SyncedToLastBlock(u32),
    SyncedToBlock(u32),
}

pub const FILTER_ID_SHIFT: u8 = 48;

impl<N: NodeRpcClient, S: Store> Client<N, S> {
    // pub async fn get_sync_height(
    //     &self,
    // ) -> Result<u32, ()> {
    //     self.store.get_sync_height().await?
    // }

    // pub async fn get_note_tags(
    //     &self
    // ) -> Result<Vec<u64>, ()> {
    //     self.store.get_note_tags().await?
    // }

    // pub async fn add_note_tag(
    //     &mut self,
    //     tag: u64,
    // ) -> Result<(), ()> {
    //     match result {
    //         Ok(true) => Ok(()),
    //         Ok(false) => {
    //             warn!("Tag {} is already being tracked", tag);
    //             Ok(())
    //         },
    //         Err(_) => Ok(()) // Ignore all errors and return Ok(()) for uniformity.
    //     }
    // }

    pub async fn sync_state(
        &mut self
    ) -> String { // TODO: Replace with Result<u32, ()>
        // self.ensure_genesis_in_place().await?;
        // loop {
        //     let response = self.sync_state_once().await?;
        //     if let SyncStatus::SyncedToLastBlock(v) = response {
        //         return Ok(v);
        //     }
        // }

        "Called sync_state".to_string()
    }

    // async fn ensure_genesis_in_place(
    //     &mut self
    // ) -> Result<(), ()> {
    //     let genesis = self.store.get_block_header_by_num(0).await?;

    //     if genesis_result.is_ok() {
    //         // If the genesis block is found, return Ok.
    //         Ok(())
    //     } else {
    //         // If there's any error (including genesis block not found), try to retrieve and store genesis.
    //         // Any error during retrieval or storage is ignored, returning Err(()).
    //         self.retrieve_and_store_genesis().await.map_err(|_| ())
    //     }
    // }

    // async fn retrieve_and_store_genesis(
    //     &mut self
    // ) -> Result<(), ()> {
    //     let genesis_block = self.rpc_api.get_block_header_by_number(Some(0)).await?;

    //     let blank_mmr_peaks =
    //         MmrPeaks::new(0, vec![]).expect("Blank MmrPeaks should not fail to instantiate");
    //     // NOTE: If genesis block data ever includes notes in the future, the third parameter in
    //     // this `insert_block_header` call may be `true`
    //     self.store.insert_block_header(genesis_block, blank_mmr_peaks, false).await?;
    //     Ok(())
    // }

    // async fn sync_state_once(
    //     &mut self
    // ) -> Result<SyncStatus, ()> {
    //     let current_block_num = self.store.get_sync_height().await?;

    //     let accounts: Vec<AccountStub> = self
    //         .store
    //         .get_account_stubs().await?
    //         .into_iter()
    //         .map(|(acc_stub, _)| acc_stub)
    //         .collect();

    //     let note_tags: Vec<u16> = accounts
    //         .iter()
    //         .map(|acc| ((u64::from(acc.id()) >> FILTER_ID_SHIFT) as u16))
    //         .collect();

    //     // To receive information about added nullifiers, we reduce them to the higher 16 bits
    //     // Note that besides filtering by nullifier prefixes, the node also filters by block number
    //     // (it only returns nullifiers from current_block_num until response.block_header.block_num())
    //     let nullifiers_tags: Vec<u16> = self
    //         .store
    //         .get_unspent_input_note_nullifiers().await?
    //         .iter()
    //         .map(|nullifier| (nullifier.inner()[3].as_int() >> FILTER_ID_SHIFT) as u16)
    //         .collect();

    //     // Send request
    //     let account_ids: Vec<AccountId> = accounts.iter().map(|acc| acc.id()).collect();
    //     let response = self
    //         .rpc_api
    //         .sync_state(current_block_num, &account_ids, &note_tags, &nullifiers_tags)
    //         .await?;

    //     // We don't need to continue if the chain has not advanced
    //     if response.block_header.block_num() == current_block_num {
    //         return Ok(SyncStatus::SyncedToLastBlock(current_block_num));
    //     }

    //     let committed_notes =
    //         self.build_inclusion_proofs(response.note_inclusions, &response.block_header).await?;

    //     // Check if the returned account hashes match latest account hashes in the database
    //     check_account_hashes(&response.account_hash_updates, &accounts)?;

    //     // Derive new nullifiers data
    //     let new_nullifiers = self.get_new_nullifiers(response.nullifiers).await?;

    //     // Build PartialMmr with current data and apply updates
    //     let (new_peaks, new_authentication_nodes) = {
    //         let current_partial_mmr = self.build_current_partial_mmr().await?;

    //         let (current_block, has_relevant_notes) =
    //             self.store.get_block_header_by_num(current_block_num).await?;

    //         apply_mmr_changes(
    //             current_partial_mmr,
    //             response.mmr_delta,
    //             current_block,
    //             has_relevant_notes,
    //         )?
    //     };

    //     let note_ids: Vec<NoteId> = committed_notes.iter().map(|(id, _)| (*id)).collect();

    //     let uncommitted_transactions =
    //         self.store.get_transactions(TransactionFilter::Uncomitted).await?;

    //     let transactions_to_commit = get_transactions_to_commit(
    //         &uncommitted_transactions,
    //         &note_ids,
    //         &new_nullifiers,
    //         &response.account_hash_updates,
    //     );

    //     // Apply received and computed updates to the store
    //     self.store
    //         .apply_state_sync(
    //             response.block_header,
    //             new_nullifiers,
    //             committed_notes,
    //             &transactions_to_commit,
    //             new_peaks,
    //             &new_authentication_nodes,
    //         ).await?;

    //     if response.chain_tip == response.block_header.block_num() {
    //         Ok(SyncStatus::SyncedToLastBlock(response.chain_tip))
    //     } else {
    //         Ok(SyncStatus::SyncedToBlock(response.block_header.block_num()))
    //     }
    // }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    // async fn build_inclusion_proofs(
    //     &self,
    //     committed_notes: Vec<CommittedNote>,
    //     block_header: &BlockHeader,
    // ) -> Result<Vec<(NoteId, NoteInclusionProof)>, ()> {
    //     // We'll only pick committed notes that we are tracking as input/output notes. Since the
    //     // sync response contains notes matching either the provided accounts or the provided tag
    //     // we might get many notes when we only care about a few of those.
    //     let pending_input_notes: Vec<NoteId> = self
    //         .store
    //         .get_input_notes(NoteFilter::Pending).await?
    //         .iter()
    //         .map(|n| n.note().id())
    //         .collect();

    //     let pending_output_notes: Vec<NoteId> = self
    //         .store
    //         .get_output_notes(NoteFilter::Pending).await?
    //         .iter()
    //         .map(|n| n.note().id())
    //         .collect();

    //     let mut pending_notes = [pending_input_notes, pending_output_notes].concat();
    //     pending_notes.dedup();

    //     committed_notes
    //         .iter()
    //         .filter_map(|commited_note| {
    //             if pending_notes.contains(commited_note.note_id()) {
    //                 // FIXME: This removal is to accomodate a problem with how the node constructs paths where
    //                 // they are constructed using note ID instead of authentication hash, so for now we remove the first
    //                 // node here.
    //                 //
    //                 // See: https://github.com/0xPolygonMiden/miden-node/blob/main/store/src/state.rs#L274
    //                 let mut merkle_path = commited_note.merkle_path().clone();
    //                 if merkle_path.len() > 0 {
    //                     let _ = merkle_path.remove(0);
    //                 }

    //                 let note_inclusion_proof = NoteInclusionProof::new(
    //                     block_header.block_num(),
    //                     block_header.sub_hash(),
    //                     block_header.note_root(),
    //                     commited_note.note_index().into(),
    //                     merkle_path,
    //                 )
    //                 .map_err(|err| ())
    //                 .map(|proof| (*commited_note.note_id(), proof));

    //                 Some(note_inclusion_proof)
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect()
    // }

    // pub(crate) async fn build_current_partial_mmr(
    //     &self
    // ) -> Result<PartialMmr, ()> {
    //     let current_block_num = self.store.get_sync_height().await?;

    //     let tracked_nodes = self.store.get_chain_mmr_nodes(ChainMmrNodeFilter::All).await?;
    //     let current_peaks = self.store.get_chain_mmr_peaks_by_block_num(current_block_num).await?;

    //     let track_latest = if current_block_num != 0 {
    //         // Attempt to fetch the block header.
    //         let result = self.store.get_block_header_by_num(current_block_num - 1).await
    //             .map(|(_, previous_block_had_notes)| previous_block_had_notes) // Directly extract the boolean if successful.
    //             .unwrap_or(false); // Return false on any error, effectively ignoring the error.
        
    //         result // Use the result as the value for track_latest.
    //     } else {
    //         false
    //     };

    //     Ok(PartialMmr::from_parts(current_peaks, tracked_nodes, track_latest))
    // }

    // async fn get_new_nullifiers(
    //     &self,
    //     new_nullifiers: Vec<Digest>,
    // ) -> Result<Vec<Digest>, ()> {
    //     let nullifiers = self
    //         .store
    //         .get_unspent_input_note_nullifiers().await?
    //         .iter()
    //         .map(|nullifier| nullifier.inner())
    //         .collect::<Vec<_>>();

    //     let new_nullifiers = new_nullifiers
    //         .into_iter()
    //         .filter(|nullifier| nullifiers.contains(nullifier))
    //         .collect();

    //     Ok(new_nullifiers)
    // }
}

// UTILS
// --------------------------------------------------------------------------------------------

// fn apply_mmr_changes(
//     current_partial_mmr: PartialMmr,
//     mmr_delta: MmrDelta,
//     current_block_header: BlockHeader,
//     current_block_has_relevant_notes: bool,
// ) -> Result<(MmrPeaks, Vec<(InOrderIndex, Digest)>), ()> {
//     let mut partial_mmr: PartialMmr = current_partial_mmr;

//     let new_authentication_nodes = partial_mmr
//         .add(current_block_header.hash(), current_block_has_relevant_notes)
//         .into_iter();

//     let new_authentication_nodes: Vec<(InOrderIndex, Digest)> = partial_mmr
//         .apply(mmr_delta)
//         .map_err(|err| ())?
//         .into_iter()
//         .chain(new_authentication_nodes)
//         .collect();

//     Ok((partial_mmr.peaks(), new_authentication_nodes))
// }

// fn check_account_hashes(
//     account_updates: &[(AccountId, Digest)],
//     current_accounts: &[AccountStub],
// ) -> Result<(), ()> {
//     for (remote_account_id, remote_account_hash) in account_updates {
//         {
//             if let Some(local_account) =
//                 current_accounts.iter().find(|acc| *remote_account_id == acc.id())
//             {
//                 if *remote_account_hash != local_account.hash() {
//                     return ();
//                 }
//             }
//         }
//     }
//     Ok(())
// }

// fn get_transactions_to_commit(
//     uncommitted_transactions: &[TransactionRecord],
//     note_ids: &[NoteId],
//     nullifiers: &[Digest],
//     account_hash_updates: &[(AccountId, Digest)],
// ) -> Vec<TransactionId> {
//     uncommitted_transactions
//         .iter()
//         .filter(|t| {
//             // TODO: based on the discussion in
//             // https://github.com/0xPolygonMiden/miden-client/issues/144, we should be aware
//             // that in the future it'll be possible to have many transactions modifying an
//             // account be included in a single block. If that happens, we'll need to rewrite
//             // this check
//             t.input_note_nullifiers.iter().all(|n| nullifiers.contains(n))
//                 && t.output_notes.iter().all(|n| note_ids.contains(&n.id()))
//                 && account_hash_updates.iter().any(|(account_id, account_hash)| {
//                     *account_id == t.account_id && *account_hash == t.final_account_state
//                 })
//         })
//         .map(|t| t.id)
//         .collect()
// }