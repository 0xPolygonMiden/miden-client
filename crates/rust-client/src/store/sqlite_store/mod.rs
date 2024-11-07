use alloc::{collections::BTreeMap, vec::Vec};
use core::cell::{RefCell, RefMut};
use std::path::Path;

use miden_objects::{
    accounts::{Account, AccountHeader, AccountId, AuthSecretKey},
    crypto::merkle::{InOrderIndex, MmrPeaks},
    notes::{NoteTag, Nullifier},
    BlockHeader, Digest, Word,
};
use rusqlite::{vtab::array, Connection};
use winter_maybe_async::maybe_async;

use self::config::SqliteStoreConfig;
use super::{
    ChainMmrNodeFilter, InputNoteRecord, NoteFilter, OutputNoteRecord, Store, TransactionFilter,
};
use crate::{
    store::StoreError,
    sync::StateSyncUpdate,
    transactions::{TransactionRecord, TransactionResult},
};

mod accounts;
mod chain_data;
pub mod config;
mod errors;
mod notes;
mod sync;
mod transactions;

// SQLITE STORE
// ================================================================================================
///
/// Represents a connection with an sqlite database
///
///
/// Current table definitions can be found at `store.sql` migration file.
pub struct SqliteStore {
    pub(crate) db: RefCell<Connection>,
}

impl SqliteStore {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Store] instantiated with the specified configuration options.
    pub fn new(config: &SqliteStoreConfig) -> Result<Self, StoreError> {
        let database_exists = Path::new(&config.database_filepath).exists();

        let db = Connection::open(config.database_filepath.clone())?;
        array::load_module(&db)?;

        if !database_exists {
            db.execute_batch(include_str!("store.sql"))?;
        }

        Ok(Self { db: RefCell::new(db) })
    }

    /// Returns a mutable reference to the internal [Connection] to the SQL DB
    pub fn db(&self) -> RefMut<'_, Connection> {
        self.db.borrow_mut()
    }
}

// SQLite implementation of the Store trait
//
// To simplify, all implementations rely on inner SqliteStore functions that map 1:1 by name
// This way, the actual implementations are grouped by entity types in their own sub-modules
impl Store for SqliteStore {
    #[maybe_async]
    fn get_note_tags(&self) -> Result<Vec<NoteTag>, StoreError> {
        self.get_note_tags()
    }

    #[maybe_async]
    fn add_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        self.add_note_tag(tag)
    }

    #[maybe_async]
    fn remove_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError> {
        self.remove_note_tag(tag)
    }

    #[maybe_async]
    fn get_sync_height(&self) -> Result<u32, StoreError> {
        self.get_sync_height()
    }

    #[maybe_async]
    fn apply_state_sync(&self, state_sync_update: StateSyncUpdate) -> Result<(), StoreError> {
        self.apply_state_sync(state_sync_update)
    }

    #[maybe_async]
    fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        self.get_transactions(transaction_filter)
    }

    #[maybe_async]
    fn apply_transaction(&self, tx_result: TransactionResult) -> Result<(), StoreError> {
        self.apply_transaction(tx_result)
    }

    #[maybe_async]
    fn get_input_notes(
        &self,
        note_filter: NoteFilter<'_>,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.get_input_notes(note_filter)
    }

    #[maybe_async]
    fn get_output_notes(
        &self,
        note_filter: NoteFilter<'_>,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        self.get_output_notes(note_filter)
    }

    #[maybe_async]
    fn insert_input_note(&self, note: InputNoteRecord) -> Result<(), StoreError> {
        self.insert_input_note(note)
    }

    #[maybe_async]
    fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        self.insert_block_header(block_header, chain_mmr_peaks, has_client_notes)
    }

    #[maybe_async]
    fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        self.get_block_headers(block_numbers)
    }

    #[maybe_async]
    fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        self.get_tracked_block_headers()
    }

    #[maybe_async]
    fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter<'_>,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        self.get_chain_mmr_nodes(filter)
    }

    #[maybe_async]
    fn insert_chain_mmr_nodes(&self, nodes: &[(InOrderIndex, Digest)]) -> Result<(), StoreError> {
        self.insert_chain_mmr_nodes(nodes)
    }

    #[maybe_async]
    fn get_chain_mmr_peaks_by_block_num(&self, block_num: u32) -> Result<MmrPeaks, StoreError> {
        self.get_chain_mmr_peaks_by_block_num(block_num)
    }

    #[maybe_async]
    fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), StoreError> {
        self.insert_account(account, account_seed, auth_info)
    }

    #[maybe_async]
    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        self.get_account_ids()
    }

    #[maybe_async]
    fn get_account_headers(&self) -> Result<Vec<(AccountHeader, Option<Word>)>, StoreError> {
        self.get_account_headers()
    }

    #[maybe_async]
    fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountHeader, Option<Word>), StoreError> {
        self.get_account_header(account_id)
    }

    #[maybe_async]
    fn get_account_header_by_hash(
        &self,
        account_hash: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        self.get_account_header_by_hash(account_hash)
    }

    #[maybe_async]
    fn get_account(&self, account_id: AccountId) -> Result<(Account, Option<Word>), StoreError> {
        self.get_account(account_id)
    }

    #[maybe_async]
    fn get_account_auth(&self, account_id: AccountId) -> Result<AuthSecretKey, StoreError> {
        self.get_account_auth(account_id)
    }

    fn get_account_auth_by_pub_key(&self, pub_key: Word) -> Result<AuthSecretKey, StoreError> {
        self.get_account_auth_by_pub_key(pub_key)
    }

    #[maybe_async]
    fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        self.get_unspent_input_note_nullifiers()
    }

    #[maybe_async]
    fn update_note_inclusion_proof(
        &self,
        note_id: miden_objects::notes::NoteId,
        inclusion_proof: miden_objects::notes::NoteInclusionProof,
    ) -> Result<(), StoreError> {
        self.update_note_inclusion_proof(note_id, inclusion_proof)
    }

    #[maybe_async]
    fn update_note_metadata(
        &self,
        note_id: miden_objects::notes::NoteId,
        metadata: miden_objects::notes::NoteMetadata,
    ) -> Result<(), StoreError> {
        self.update_note_metadata(note_id, metadata)
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use std::cell::RefCell;

    use rusqlite::{vtab::array, Connection};

    use super::SqliteStore;
    use crate::mock::create_test_store_path;

    pub(crate) fn create_test_store() -> SqliteStore {
        let temp_file = create_test_store_path();
        let db = Connection::open(temp_file).unwrap();
        array::load_module(&db).unwrap();
        db.execute_batch(include_str!("store.sql")).unwrap();
        SqliteStore { db: RefCell::new(db) }
    }
}
