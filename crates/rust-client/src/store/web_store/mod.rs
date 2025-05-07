//! Provides an IndexedDB-backed implementation of the [Store] trait for web environments.
//!
//! This module enables persistence of client data (accounts, transactions, notes, block headers,
//! etc.) when running in a browser. It uses wasm-bindgen to interface with JavaScript and
//! `IndexedDB`, allowing the Miden client to store and retrieve data asynchronously.
//!
//! **Note:** This implementation is only available when targeting WebAssembly with the `web_store`
//! feature enabled.

use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use miden_objects::{
    Digest, Word,
    account::{Account, AccountCode, AccountHeader, AccountId},
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InOrderIndex, MmrPeaks},
    note::Nullifier,
    transaction::TransactionId,
};
use tonic::async_trait;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, js_sys, wasm_bindgen};

use super::{
    AccountRecord, AccountStatus, InputNoteRecord, NoteFilter, OutputNoteRecord,
    PartialBlockchainFilter, Store, StoreError, TransactionFilter,
};
use crate::{
    note::NoteUpdates,
    sync::{NoteTagRecord, StateSyncUpdate},
    transaction::{TransactionRecord, TransactionStoreUpdate},
};

#[cfg(not(target_arch = "wasm32"))]
compile_error!("The `idxdb` feature is only supported when targeting wasm32.");

pub mod account;
pub mod chain_data;
pub mod export;
pub mod import;
pub mod note;
pub mod sync;
pub mod transaction;

// Initialize IndexedDB
#[wasm_bindgen(module = "/src/store/web_store/js/schema.js")]
extern "C" {
    #[wasm_bindgen(js_name = openDatabase)]
    fn setup_indexed_db() -> js_sys::Promise;
}

pub struct WebStore {}

impl WebStore {
    pub async fn new() -> Result<WebStore, JsValue> {
        JsFuture::from(setup_indexed_db()).await?;
        Ok(WebStore {})
    }
}

#[async_trait(?Send)]
impl Store for WebStore {
    fn get_current_timestamp(&self) -> Option<u64> {
        let now = chrono::Utc::now();
        Some(u64::try_from(now.timestamp()).expect("timestamp is always after epoch"))
    }

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

    async fn get_sync_height(&self) -> Result<BlockNumber, StoreError> {
        self.get_sync_height().await
    }

    async fn apply_state_sync(&self, state_sync_update: StateSyncUpdate) -> Result<(), StoreError> {
        self.apply_state_sync(state_sync_update).await
    }

    async fn apply_nullifiers(
        &self,
        note_updates: NoteUpdates,
        transactions_to_discard: Vec<TransactionId>,
    ) -> Result<(), StoreError> {
        self.apply_nullifiers(note_updates, transactions_to_discard).await
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
        block_header: &BlockHeader,
        partial_blockchain_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        self.insert_block_header(block_header, partial_blockchain_peaks, has_client_notes)
            .await
    }

    async fn get_block_headers(
        &self,
        block_numbers: &BTreeSet<BlockNumber>,
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        self.get_block_headers(block_numbers).await
    }

    async fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        self.get_tracked_block_headers().await
    }

    async fn get_partial_blockchain_nodes(
        &self,
        filter: PartialBlockchainFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        self.get_partial_blockchain_nodes(filter).await
    }

    async fn insert_partial_blockchain_nodes(
        &self,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        self.insert_partial_blockchain_nodes(nodes).await
    }

    async fn get_partial_blockchain_peaks_by_block_num(
        &self,
        block_num: BlockNumber,
    ) -> Result<MmrPeaks, StoreError> {
        self.get_partial_blockchain_peaks_by_block_num(block_num).await
    }

    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

    async fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
    ) -> Result<(), StoreError> {
        self.insert_account(account, account_seed).await
    }

    async fn update_account(&self, new_account_state: &Account) -> Result<(), StoreError> {
        self.update_account(new_account_state).await
    }

    async fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        self.get_account_ids().await
    }

    async fn get_account_headers(&self) -> Result<Vec<(AccountHeader, AccountStatus)>, StoreError> {
        self.get_account_headers().await
    }

    async fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<Option<(AccountHeader, AccountStatus)>, StoreError> {
        self.get_account_header(account_id).await
    }

    async fn get_account_header_by_commitment(
        &self,
        account_commitment: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        self.get_account_header_by_commitment(account_commitment).await
    }

    async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountRecord>, StoreError> {
        self.get_account(account_id).await
    }

    async fn upsert_foreign_account_code(
        &self,
        account_id: AccountId,
        code: AccountCode,
    ) -> Result<(), StoreError> {
        self.upsert_foreign_account_code(account_id, code).await
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
