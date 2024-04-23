use async_trait::async_trait;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::*;

use miden_objects::{
    accounts::{Account, AccountId, AccountStub}, notes::NoteId, Word
};

use crate::native_code::{
    errors::StoreError, 
    store::{
        note_record::{InputNoteRecord, OutputNoteRecord}, AuthInfo, NoteFilter, Store
    }
}; 

pub mod accounts;
pub mod notes;
pub mod transactions;
pub mod sync;
pub mod chain_data;
pub mod mock_example;

// Initialize IndexedDB
#[wasm_bindgen(module = "/js/db/schema.js")]
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
    // TEST FUNCTION
    async fn insert_string(
        &mut self, 
        data: String
    ) -> Result<(), ()> {
        self.insert_string(data).await
    }

    // CHAIN DATA
    // --------------------------------------------------------------------------------------------

    // async fn get_block_headers(
    //     &self,
    //     block_numbers: &[u32],
    // ) -> Result<Vec<(BlockHeader, bool)>, ()> {
    //     self.get_block_headers(block_numbers).await
    // }

    // async fn insert_block_header(
    //     &self,
    //     block_header: BlockHeader,
    //     chain_mmr_peaks: MmrPeaks,
    //     has_client_notes: bool,
    // ) -> Result<(), ()> {
    //     self.insert_block_header(block_header, chain_mmr_peaks, has_client_notes).await
    // }

    // async fn get_tracked_block_headers(
    //     &self
    // ) -> Result<Vec<BlockHeader>, ()> {
    //     self.get_tracked_block_headers().await
    // }

    // async fn get_chain_mmr_nodes(
    //     &self,
    //     filter: ChainMmrNodeFilter,
    // ) -> Result<BTreeMap<InOrderIndex, Digest>, ()> {
    //     self.get_chain_mmr_nodes(filter).await
    // }

    // async fn get_chain_mmr_peaks_by_block_num(
    //     &self, 
    //     block_num: u32
    // ) -> Result<MmrPeaks, ()> {
    //     self.get_chain_mmr_peaks_by_block_num(block_num).await
    // }

    // SYNC
    // --------------------------------------------------------------------------------------------

    // async fn get_note_tags(
    //     &self
    // ) -> Result<Vec<u64>, ()> {
    //     self.get_note_tags().await
    // }

    // async fn add_note_tag(
    //     &mut self,
    //     tag: u64,
    // ) -> Result<bool, ()> {
    //     self.add_note_tag(tag).await
    // }

    // async fn get_sync_height(
    //     &self
    // ) -> Result<u32, ()> {
    //     self.get_sync_height().await
    // }

    // async fn apply_state_sync(
    //     &mut self,
    //     block_header: BlockHeader,
    //     nullifiers: Vec<Digest>,
    //     committed_notes: Vec<(NoteId, NoteInclusionProof)>,
    //     committed_transactions: &[TransactionId],
    //     new_mmr_peaks: MmrPeaks,
    //     new_authentication_nodes: &[(InOrderIndex, Digest)],
    // ) -> Result<(), ()> {
    //     self.apply_state_sync(
    //         block_header,
    //         nullifiers,
    //         committed_notes,
    //         committed_transactions,
    //         new_mmr_peaks,
    //         new_authentication_nodes,
    //     ).await
    // }

    // TRANSACTIONS
    // --------------------------------------------------------------------------------------------

    // async fn get_transactions(
    //     &mut self,
    //     transaction_filter: TransactionFilter,
    // ) -> Result<Vec<TransactionRecord>, ()> {
    //     self.get_transactions(transaction_filter).await
    // }

    // async fn apply_transaction(
    //     &mut self,
    //     tx_result: TransactionResult,
    // ) -> Result<(), ()> {
    //     self.apply_transaction(tx_result).await
    // }

    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

    async fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthInfo,
    ) -> Result<(), ()> {
        self.insert_account(account, account_seed, auth_info).await
    }

    async fn get_account_ids(
        &mut self
    ) -> Result<Vec<AccountId>, ()> {
        self.get_account_ids().await
    }

    async fn get_account_stubs(
        &mut self
    ) -> Result<Vec<(AccountStub, Option<Word>)>, ()> {
        self.get_account_stubs().await
    }

    async fn get_account_stub(
        &mut self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), ()> {
        self.get_account_stub(account_id).await
    }

    async fn get_account(
        &mut self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), ()> {
        self.get_account(account_id).await
    }

    async fn get_account_auth(
        &mut self,
        account_id: AccountId,
    ) -> Result<AuthInfo, ()> {
        self.get_account_auth(account_id).await
    }

    // NOTES
    // --------------------------------------------------------------------------------------------

    async fn get_input_notes(
        &mut self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.get_input_notes(filter).await
    }

    async fn get_input_note(
        &mut self,
        note_id: NoteId,
    ) -> Result<InputNoteRecord, StoreError> {
        self.get_input_note(note_id).await
    }

    async fn insert_input_note(
        &mut self,
        note: &InputNoteRecord,
    ) -> Result<(), StoreError> {
        self.insert_input_note(note).await
    }

    async fn get_output_notes(
        &mut self,
        note_filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        self.get_output_notes(note_filter).await
    }
}