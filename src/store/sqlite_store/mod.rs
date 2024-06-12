use alloc::collections::BTreeMap;
use core::cell::{RefCell, RefMut};

use miden_objects::{
    accounts::{Account, AccountId, AccountStub, AuthSecretKey},
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
    client::{
        sync::StateSyncUpdate,
        transactions::{TransactionRecord, TransactionResult},
    },
    errors::StoreError,
};

mod accounts;
mod chain_data;
pub mod config;
pub(crate) mod migrations;
mod notes;
mod sync;
mod transactions;

// SQLITE STORE
// ================================================================================================
///
/// Represents a connection with an sqlite database
///
///
/// Current table definitions can be found at `store.sql` migration file. One particular column
/// type used is JSON, for which you can look more info at [sqlite's official documentation](https://www.sqlite.org/json1.html).
/// In the case of json, some caveats must be taken:
///
/// - To insert json values you must use sqlite's `json` function in the query alongside named
///   parameters, and the provided parameter must be a valid json. That is:
///
/// ```sql
/// INSERT INTO SOME_TABLE
///     (some_field)
///     VALUES (json(:some_field))")
/// ```
///
/// ```ignore
/// let metadata = format!(r#"{{"some_inner_field": {some_field}, "some_other_inner_field": {some_other_field}}}"#);
/// ```
///
/// (Using raw string literals for the jsons is encouraged if possible)
///
/// - To get data from any of the json fields you can use the `json_extract` function (in some
///   cases you'll need to do some explicit type casting to help rusqlite figure out types):
///
/// ```sql
/// SELECT CAST(json_extract(some_json_col, '$.some_json_field') AS TEXT) from some_table
/// ```
///
/// - For some datatypes you'll need to do some manual serialization/deserialization. For example,
///   suppose one of your json fields is an array of digests. Then you'll need to
///     - Create the json with an array of strings representing the digests:
///
///     ```ignore
///     let some_array_field = some_array
///         .into_iter()
///         .map(array_elem_to_string)
///         .collect::<Vec<_>>()
///         .join(",");
///
///     Some(format!(
///         r#"{{
///             "some_array_field": [{some_array_field}]
///         }}"#
///     )),
///     ```
///
///     - When deserializing, handling the extra symbols (`[`, `]`, `,`, `"`). For that you can use
///       the `parse_json_array` function:
///
///     ```ignore
///         let some_array = parse_json_array(some_array_field)
///         .into_iter()
///         .map(parse_json_byte_str)
///         .collect::<Result<Vec<u8>, _>>()?;
///     ```
/// - Thus, if needed you can create a struct representing the json values and use serde_json to
///   simplify all of the serialization/deserialization logic
pub struct SqliteStore {
    pub(crate) db: RefCell<Connection>,
}

impl SqliteStore {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Store] instantiated with the specified configuration options.
    pub fn new(config: &SqliteStoreConfig) -> Result<Self, StoreError> {
        let mut db = Connection::open(config.database_filepath.clone())?;
        array::load_module(&db)?;
        migrations::update_to_latest(&mut db)?;

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
    fn get_account_stubs(&self) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError> {
        self.get_account_stubs()
    }

    #[maybe_async]
    fn get_account_stub(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError> {
        self.get_account_stub(account_id)
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
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use std::cell::RefCell;

    use rusqlite::{vtab::array, Connection};

    use super::{migrations, SqliteStore};
    use crate::mock::create_test_store_path;

    pub(crate) fn create_test_store() -> SqliteStore {
        let temp_file = create_test_store_path();
        let mut db = Connection::open(temp_file).unwrap();
        array::load_module(&db).unwrap();
        migrations::update_to_latest(&mut db).unwrap();

        SqliteStore { db: RefCell::new(db) }
    }
}
