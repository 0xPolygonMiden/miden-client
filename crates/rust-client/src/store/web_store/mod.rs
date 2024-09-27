use alloc::{collections::BTreeMap, vec::Vec};

#[cfg(feature = "async")]
use async_trait::async_trait;
use miden_objects::{
    accounts::{Account, AccountHeader, AccountId, AuthSecretKey},
    crypto::merkle::{InOrderIndex, MmrPeaks},
    notes::{NoteTag, Nullifier},
    BlockHeader, Digest, Word,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;
use winter_maybe_async::{maybe_async, maybe_await};

use crate::{
    store::{
        ChainMmrNodeFilter, InputNoteRecord, NoteFilter, OutputNoteRecord, Store, StoreError,
        TransactionFilter,
    },
    sync::StateSyncUpdate,
    transactions::{TransactionRecord, TransactionResult},
};

pub mod accounts;
pub mod chain_data;
pub mod notes;
pub mod sync;
pub mod transactions;

// Initialize IndexedDB
#[wasm_bindgen(module = "/src/store/web_store/js/schema.js")]
extern "C" {
    #[wasm_bindgen(js_name = openDatabase)]
    fn setup_indexed_db() -> js_sys::Promise;
}

pub struct WebStore {}

impl WebStore {
    pub async fn new() -> Result<WebStore, ()> {
        let _ = JsFuture::from(setup_indexed_db()).await;
        Ok(WebStore {})
    }
}

#[cfg_attr(feature = "async", async_trait)]
impl Store for WebStore {
    // SYNC
    // --------------------------------------------------------------------------------------------

    #[maybe_async]
    fn get_note_tags(&self) -> Result<Vec<NoteTag>, StoreError> {
        maybe_await!(self.get_note_tags())
    }

    #[maybe_async]
    fn add_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        maybe_await!(self.add_note_tag(tag))
    }

    #[maybe_async]
    fn remove_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        maybe_await!(self.remove_note_tag(tag))
    }

    #[maybe_async]
    fn get_sync_height(&self) -> Result<u32, StoreError> {
        maybe_await!(self.get_sync_height())
    }

    #[maybe_async]
    fn apply_state_sync(&self, state_sync_update: StateSyncUpdate) -> Result<(), StoreError> {
        maybe_await!(self.apply_state_sync(state_sync_update))
    }

    // TRANSACTIONS
    // --------------------------------------------------------------------------------------------

    #[maybe_async]
    fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        maybe_await!(self.get_transactions(transaction_filter))
    }

    #[maybe_async]
    fn apply_transaction(&self, tx_result: TransactionResult) -> Result<(), StoreError> {
        maybe_await!(self.apply_transaction(tx_result))
    }

    // NOTES
    // --------------------------------------------------------------------------------------------

    #[maybe_async]
    fn get_input_notes(&self, filter: NoteFilter<'_>) -> Result<Vec<InputNoteRecord>, StoreError> {
        maybe_await!(self.get_input_notes(filter))
    }

    #[maybe_async]
    fn get_output_notes(
        &self,
        note_filter: NoteFilter<'_>,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        maybe_await!(self.get_output_notes(note_filter))
    }

    #[maybe_async]
    fn insert_input_note(&self, note: InputNoteRecord) -> Result<(), StoreError> {
        maybe_await!(self.insert_input_note(note))
    }

    #[maybe_async]
    fn update_note_inclusion_proof(
        &self,
        note_id: miden_objects::notes::NoteId,
        inclusion_proof: miden_objects::notes::NoteInclusionProof,
    ) -> Result<(), StoreError> {
        maybe_await!(self.update_note_inclusion_proof(note_id, inclusion_proof))
    }

    #[maybe_async]
    fn update_note_metadata(
        &self,
        note_id: miden_objects::notes::NoteId,
        metadata: miden_objects::notes::NoteMetadata,
    ) -> Result<(), StoreError> {
        maybe_await!(self.update_note_metadata(note_id, metadata))
    }

    // CHAIN DATA
    // --------------------------------------------------------------------------------------------

    #[maybe_async]
    fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        maybe_await!(self.insert_block_header(block_header, chain_mmr_peaks, has_client_notes))
    }

    #[maybe_async]
    fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        maybe_await!(self.get_block_headers(block_numbers))
    }

    #[maybe_async]
    fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        maybe_await!(self.get_tracked_block_headers())
    }

    #[maybe_async]
    fn get_chain_mmr_nodes<'a>(
        &self,
        filter: ChainMmrNodeFilter<'a>,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        maybe_await!(self.get_chain_mmr_nodes(filter))
    }

    #[maybe_async]
    fn insert_chain_mmr_nodes(&self, nodes: &[(InOrderIndex, Digest)]) -> Result<(), StoreError> {
        maybe_await!(self.insert_chain_mmr_nodes(nodes))
    }

    #[maybe_async]
    fn get_chain_mmr_peaks_by_block_num(&self, block_num: u32) -> Result<MmrPeaks, StoreError> {
        maybe_await!(self.get_chain_mmr_peaks_by_block_num(block_num))
    }

    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

    #[maybe_async]
    fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), StoreError> {
        maybe_await!(self.insert_account(account, account_seed, auth_info))
    }

    #[maybe_async]
    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        maybe_await!(self.get_account_ids())
    }

    #[maybe_async]
    fn get_account_headers(&self) -> Result<Vec<(AccountHeader, Option<Word>)>, StoreError> {
        maybe_await!(self.get_account_headers())
    }

    #[maybe_async]
    fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountHeader, Option<Word>), StoreError> {
        maybe_await!(self.get_account_header(account_id))
    }

    #[maybe_async]
    fn get_account_header_by_hash(
        &self,
        account_hash: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        maybe_await!(self.get_account_header_by_hash(account_hash))
    }

    #[maybe_async]
    fn get_account(&self, account_id: AccountId) -> Result<(Account, Option<Word>), StoreError> {
        maybe_await!(self.get_account(account_id))
    }

    #[maybe_async]
    fn get_account_auth(&self, account_id: AccountId) -> Result<AuthSecretKey, StoreError> {
        maybe_await!(self.get_account_auth(account_id))
    }

    fn get_account_auth_by_pub_key(&self, pub_key: Word) -> Result<AuthSecretKey, StoreError> {
        self.get_account_auth_by_pub_key(pub_key)
    }

    #[maybe_async]
    fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        maybe_await!(self.get_unspent_input_note_nullifiers())
    }
}
