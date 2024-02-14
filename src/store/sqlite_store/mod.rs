use objects::{
    accounts::{Account, AccountId, AccountStub},
    notes::NoteInclusionProof,
    utils::collections::BTreeMap,
    Digest,
};

use super::{
    AuthInfo, ChainMmrNodeFilter, InputNoteFilter, InputNoteRecord, Store, TransactionFilter,
};
use crypto::{
    merkle::{InOrderIndex, MmrPeaks},
    Word,
};
use objects::{notes::NoteId, BlockHeader};

use rusqlite::Connection;

use crate::{
    client::transactions::{TransactionRecord, TransactionResult},
    config::StoreConfig,
    errors::StoreError,
};

mod accounts;
mod chain_data;
mod migrations;
mod notes;
mod sync;
mod transactions;

// SQLITE STORE
// ================================================================================================

pub struct SqliteStore {
    pub(crate) db: Connection,
}

impl SqliteStore {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Store] instantiated with the specified configuration options.
    pub fn new(config: StoreConfig) -> Result<Self, StoreError> {
        let mut db = Connection::open(config.database_filepath)?;
        migrations::update_to_latest(&mut db)?;

        Ok(Self { db })
    }
}

// SQLite implementation of the Store trait
//
// To simplify, all implementations rely on inner SqliteStore functions that map 1:1 by name
// This way, the actual implementations are grouped by entity types in their own sub-modules
impl Store for SqliteStore {
    fn get_note_tags(&self) -> Result<Vec<u64>, StoreError> {
        self.get_note_tags()
    }

    fn add_note_tag(&mut self, tag: u64) -> Result<bool, StoreError> {
        self.add_note_tag(tag)
    }

    fn get_sync_height(&self) -> Result<u32, StoreError> {
        self.get_sync_height()
    }

    fn apply_state_sync(
        &mut self,
        block_header: BlockHeader,
        nullifiers: Vec<Digest>,
        committed_notes: Vec<(NoteId, NoteInclusionProof)>,
        new_mmr_peaks: MmrPeaks,
        new_authentication_nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        self.apply_state_sync(
            block_header,
            nullifiers,
            committed_notes,
            new_mmr_peaks,
            new_authentication_nodes,
        )
    }

    fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        self.get_transactions(transaction_filter)
    }

    fn insert_transaction_data(
        &mut self,
        tx_result: TransactionResult,
        updated_account: Account,
        account_seed: Option<Word>,
        created_notes: &[InputNoteRecord],
    ) -> Result<(), StoreError> {
        self.insert_transaction_data(tx_result, updated_account, account_seed, created_notes)
    }

    fn get_input_notes(
        &self,
        note_filter: InputNoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.get_input_notes(note_filter)
    }

    fn get_input_note_by_id(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError> {
        self.get_input_note_by_id(note_id)
    }

    fn insert_input_note(&mut self, note: &InputNoteRecord) -> Result<(), StoreError> {
        self.insert_input_note(note)
    }

    fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Digest>, StoreError> {
        self.get_unspent_input_note_nullifiers()
    }

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

    fn get_block_header_by_num(
        &self,
        block_number: u32,
    ) -> Result<(BlockHeader, bool), StoreError> {
        self.get_block_header_by_num(block_number)
    }

    fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        self.get_tracked_block_headers()
    }

    fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        self.get_chain_mmr_nodes(filter)
    }

    fn get_chain_mmr_peaks_by_block_num(&self, block_num: u32) -> Result<MmrPeaks, StoreError> {
        self.get_chain_mmr_peaks_by_block_num(block_num)
    }

    fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Word,
        auth_info: &AuthInfo,
    ) -> Result<(), StoreError> {
        self.insert_account(account, account_seed, auth_info)
    }

    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        self.get_account_ids()
    }

    fn get_accounts(&self) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError> {
        self.get_accounts()
    }

    fn get_account_stub_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError> {
        self.get_account_stub_by_id(account_id)
    }

    fn get_account_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), StoreError> {
        self.get_account_by_id(account_id)
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use std::env::temp_dir;
    use uuid::Uuid;

    use rusqlite::Connection;

    use crate::{
        client::Client,
        config::{ClientConfig, RpcConfig},
    };

    use super::{migrations, SqliteStore};

    pub fn create_test_client() -> Client {
        let client_config = ClientConfig {
            store: create_test_store_path()
                .into_os_string()
                .into_string()
                .unwrap()
                .try_into()
                .unwrap(),
            rpc: RpcConfig::default(),
        };

        Client::new(client_config).unwrap()
    }

    pub(crate) fn create_test_store_path() -> std::path::PathBuf {
        let mut temp_file = temp_dir();
        temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
        temp_file
    }

    pub(crate) fn create_test_store() -> SqliteStore {
        let temp_file = create_test_store_path();
        let mut db = Connection::open(temp_file).unwrap();
        migrations::update_to_latest(&mut db).unwrap();

        SqliteStore { db }
    }
}
