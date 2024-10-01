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

#[maybe_async]
impl Store for WebStore {
    // SYNC
    // --------------------------------------------------------------------------------------------

    fn get_note_tags(&self) -> Result<Vec<NoteTag>, StoreError> {
        self.get_note_tags()
    }

    fn add_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        self.add_note_tag(tag)
    }

    fn remove_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        self.remove_note_tag(tag)
    }

    fn get_sync_height(&self) -> Result<u32, StoreError> {
        self.get_sync_height()
    }

    fn apply_state_sync(&self, state_sync_update: StateSyncUpdate) -> Result<(), StoreError> {
        self.apply_state_sync(state_sync_update)
    }

    // TRANSACTIONS
    // --------------------------------------------------------------------------------------------

    fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        self.get_transactions(transaction_filter)
    }

    fn apply_transaction(&self, tx_result: TransactionResult) -> Result<(), StoreError> {
        self.apply_transaction(tx_result)
    }

    // NOTES
    // --------------------------------------------------------------------------------------------

    fn get_input_notes(&self, filter: NoteFilter<'_>) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.get_input_notes(filter)
    }

    fn get_output_notes(
        &self,
        note_filter: NoteFilter<'_>,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        self.get_output_notes(note_filter)
    }

    fn insert_input_note(&self, note: InputNoteRecord) -> Result<(), StoreError> {
        self.insert_input_note(note)
    }

    fn update_note_inclusion_proof(
        &self,
        note_id: miden_objects::notes::NoteId,
        inclusion_proof: miden_objects::notes::NoteInclusionProof,
    ) -> Result<(), StoreError> {
        self.update_note_inclusion_proof(note_id, inclusion_proof)
    }

    fn update_note_metadata(
        &self,
        note_id: miden_objects::notes::NoteId,
        metadata: miden_objects::notes::NoteMetadata,
    ) -> Result<(), StoreError> {
        self.update_note_metadata(note_id, metadata)
    }

    // CHAIN DATA
    // --------------------------------------------------------------------------------------------

    fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        self.insert_block_header(block_header, chain_mmr_peaks, has_client_notes)
    }

    fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        self.get_block_headers(block_numbers)
    }

    fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        self.get_tracked_block_headers()
    }

    fn get_chain_mmr_nodes<'a>(
        &self,
        filter: ChainMmrNodeFilter<'a>,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        self.get_chain_mmr_nodes(filter)
    }

    fn insert_chain_mmr_nodes(&self, nodes: &[(InOrderIndex, Digest)]) -> Result<(), StoreError> {
        self.insert_chain_mmr_nodes(nodes)
    }

    fn get_chain_mmr_peaks_by_block_num(&self, block_num: u32) -> Result<MmrPeaks, StoreError> {
        self.get_chain_mmr_peaks_by_block_num(block_num)
    }

    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

    fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), StoreError> {
        self.insert_account(account, account_seed, auth_info)
    }

    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        self.get_account_ids()
    }

    fn get_account_headers(&self) -> Result<Vec<(AccountHeader, Option<Word>)>, StoreError> {
        self.get_account_headers()
    }

    fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountHeader, Option<Word>), StoreError> {
        self.get_account_header(account_id)
    }

    fn get_account_header_by_hash(
        &self,
        account_hash: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        self.get_account_header_by_hash(account_hash)
    }

    fn get_account(&self, account_id: AccountId) -> Result<(Account, Option<Word>), StoreError> {
        self.get_account(account_id)
    }

    fn get_account_auth(&self, account_id: AccountId) -> Result<AuthSecretKey, StoreError> {
        self.get_account_auth(account_id)
    }

    fn get_account_auth_by_pub_key(&self, pub_key: Word) -> Result<AuthSecretKey, StoreError> {
        self.get_account_auth_by_pub_key(pub_key)
    }

    fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        self.get_unspent_input_note_nullifiers()
    }
}
