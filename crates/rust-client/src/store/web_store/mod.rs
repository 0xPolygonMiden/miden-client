use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

use miden_objects::{
    accounts::{Account, AccountCode, AccountHeader, AccountId, AuthSecretKey},
    crypto::merkle::{InOrderIndex, MmrPeaks},
    notes::Nullifier,
    BlockHeader, Digest, Word,
};
use tonic::async_trait;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

use super::{
    ChainMmrNodeFilter, InputNoteRecord, NoteFilter, OutputNoteRecord, Store, StoreError,
    TransactionFilter,
};
use crate::{
    sync::{NoteTagRecord, StateSyncUpdate},
    transactions::{TransactionRecord, TransactionStoreUpdate},
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
#[async_trait(?Send)]
impl Store for WebStore {
    // SYNC
    // --------------------------------------------------------------------------------------------
    async fn get_note_tags(&self) -> Result<Vec<NoteTagRecord>, StoreError> {
        self.get_note_tags().await
    }

    async fn add_note_tag(&self, tag: NoteTagRecord) -> Result<bool, StoreError> {
        self.add_note_tag(tag).await
    }

    async fn remove_note_tag(&self, tag: NoteTagRecord) -> Result<usize, StoreError> {
        self.remove_note_tag(tag).await
    }

    async fn get_sync_height(&self) -> Result<u32, StoreError> {
        self.get_sync_height().await
    }

    async fn apply_state_sync(&self, state_sync_update: StateSyncUpdate) -> Result<(), StoreError> {
        self.apply_state_sync(state_sync_update).await
    }

    // TRANSACTIONS
    // --------------------------------------------------------------------------------------------

    async fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        self.get_transactions(transaction_filter).await
    }

    async fn apply_transaction(&self, tx_update: TransactionStoreUpdate) -> Result<(), StoreError> {
        self.apply_transaction(tx_update).await
    }

    // NOTES
    // --------------------------------------------------------------------------------------------
    async fn get_input_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.get_input_notes(filter).await
    }

    async fn get_output_notes(
        &self,
        note_filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        self.get_output_notes(note_filter).await
    }

    async fn upsert_input_notes(&self, notes: &[InputNoteRecord]) -> Result<(), StoreError> {
        self.upsert_input_notes(notes).await
    }

    // CHAIN DATA
    // --------------------------------------------------------------------------------------------

    async fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        self.insert_block_header(block_header, chain_mmr_peaks, has_client_notes).await
    }

    async fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        self.get_block_headers(block_numbers).await
    }

    async fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        self.get_tracked_block_headers().await
    }

    async fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        self.get_chain_mmr_nodes(filter).await
    }

    async fn insert_chain_mmr_nodes(
        &self,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        self.insert_chain_mmr_nodes(nodes).await
    }

    async fn get_chain_mmr_peaks_by_block_num(
        &self,
        block_num: u32,
    ) -> Result<MmrPeaks, StoreError> {
        self.get_chain_mmr_peaks_by_block_num(block_num).await
    }

    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

    async fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), StoreError> {
        self.insert_account(account, account_seed, auth_info).await
    }

    async fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        self.get_account_ids().await
    }

    async fn get_account_auth_by_pub_key(
        &self,
        pub_key: Word,
    ) -> Result<AuthSecretKey, StoreError> {
        self.get_account_auth_by_pub_key(pub_key)
    }

    async fn get_account_headers(&self) -> Result<Vec<(AccountHeader, Option<Word>)>, StoreError> {
        self.get_account_headers().await
    }

    async fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountHeader, Option<Word>), StoreError> {
        self.get_account_header(account_id).await
    }

    async fn get_account_header_by_hash(
        &self,
        account_hash: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        self.get_account_header_by_hash(account_hash).await
    }

    async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), StoreError> {
        self.get_account(account_id).await
    }

    async fn get_account_auth(&self, account_id: AccountId) -> Result<AuthSecretKey, StoreError> {
        self.get_account_auth(account_id).await
    }

    async fn upsert_foreign_account_code(
        &self,
        account_id: AccountId,
        code: AccountCode,
    ) -> Result<(), StoreError> {
        self.update_foreign_account_code(account_id, code).await
    }

    async fn get_foreign_account_code(
        &self,
        account_ids: Vec<AccountId>,
    ) -> Result<BTreeMap<AccountId, AccountCode>, StoreError> {
        self.get_foreign_account_code(account_ids).await
    }

    async fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        self.get_unspent_input_note_nullifiers().await
    }
}
