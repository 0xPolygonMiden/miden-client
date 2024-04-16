// use wasm_bindgen_futures::*;
// use serde_wasm_bindgen::from_value;

use super::WebStore;

mod js_bindings;
// use js_bindings::*;

mod models;
// use models::*;

mod utils;
// use utils::*;

impl WebStore {
    pub(crate) async fn get_note_tags(
        &self
    ) -> Result<Vec<u64>, ()>{
        let promsie = idxdb_get_note_tags();
        let js_value = JsFuture::from(promsie).await?;
        let tags_idxdb: NoteTagsIdxdbObject = from_value(js_value).unwrap();

        let tags: Vec<u64> = serde_json::from_str(&tags_idxdb.tags).unwrap();

        return tags;
    }

    pub(super) async fn get_sync_height(
        &self
    ) -> Result<u32, ()> {
        let promise = idxdb_get_sync_height();
        let js_value = JsFuture::from(promise).await?;
        let block_num_idxdb: SyncHeightIdxdbObject = from_value(js_value).unwrap();

        let block_num_as_u32: u32 = block_num_idxdb.block_num.parse::<u32>().unwrap();
        return block_num_as_u32;
    }

    pub(super) async fn add_note_tag(
        &mut self,
        tag: u64
    ) -> Result<bool, ()> {
        let mut tags = self.get_note_tags().await?;
        if tags.contains(&tag) {
            return Ok(false);
        }
        tags.push(tag);
        let tags = serde_json::to_string(&tags)?;

        let promise = idxdb_add_note_tag(tags);
        let _ = JsFuture::from(promise).await?;
        return Ok(true);
    }

    pub(super) async fn apply_state_sync(
        &mut self,
        block_header: BlockHeader,
        nullifiers: Vec<Digest>,
        committed_notes: Vec<(NoteId, NoteInclusionProof)>,
        committed_transactions: &[TransactionId],
        new_mmr_peaks: MmrPeaks,
        new_authentication_nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), ()> {
        let block_num_as_str = block_header.block_num().to_string();
        let nullifiers_as_str = nullifiers.iter().map(|nullifier| nullifier.to_hex()).collect();
        let note_ids_as_str: Vec<String> = committed_notes.iter().map(|(note_id, _)| note_id.inner().to_hex()).collect();
        let inclusion_proofs_as_str: Vec<String> = committed_notes.iter().map(|(_, inclusion_proof)| { 
            let block_num = inclusion_proof.origin().block_num;
            let sub_hash = inclusion_proof.sub_hash();
            let note_root = inclusion_proof.note_root();
            let note_index = inclusion_proof.origin().node_index.value();

            serde_json::to_string(&NoteInclusionProof::new(
                block_num,
                sub_hash,
                note_root,
                note_index,
                inclusion_proof.note_path().clone(),
            )).unwrap()
        }).collect();
        let transactions_to_commit_as_str: Vec<String> = committed_transactions.iter().map(|tx_id| tx_id.inner().into()).collect();

        let promise = idxdb_apply_state_sync(
            block_num_as_str,
            nullifiers_as_str,
            note_ids_as_str,
            inclusion_proofs_as_str,
            transactions_to_commit_as_str,
            // new_mmr_peaks,
            // new_authentication_nodes,
        );
        let _ = JsFuture::from(promise).await?;

        // TODO: HANDLE THESE INSERTS IN JS
        // let block_has_relevant_notes = !committed_notes.is_empty();
        // Self::insert_block_header_tx(&tx, block_header, new_mmr_peaks, block_has_relevant_notes)?;

        // Self::insert_chain_mmr_nodes(&tx, new_authentication_nodes)?;

        Ok(())
    }
}