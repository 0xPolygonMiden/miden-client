use alloc::collections::BTreeMap;

use miden_objects::{
    accounts::{Account, AccountId, AccountStub},
    crypto::merkle::{InOrderIndex, MmrPeaks},
    transaction::TransactionId,
    BlockHeader, Digest, Word,
};
use rusqlite::Connection;

use super::{
    AuthInfo, ChainMmrNodeFilter, InputNoteRecord, NoteFilter, OutputNoteRecord, Store,
    TransactionFilter,
};
use crate::{
    client::{
        sync::SyncedNewNotes,
        transactions::{TransactionRecord, TransactionResult},
    },
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
///
/// Represents a connection with an sqlite database
///
///
/// Current table definitions can be found at `store.sql` migration file. One particular column
/// type used is JSON, for which you can look more info at [sqlite's official documentation](https://www.sqlite.org/json1.html).
/// In the case of json, some caveats must be taken:
///
/// - To insert json values you must use sqlite's `json` function in the query alongside named
/// parameters, and the provided parameter must be a valid json. That is:
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
/// cases you'll need to do some explicit type casting to help rusqlite figure out types):
///
/// ```sql
/// SELECT CAST(json_extract(some_json_col, '$.some_json_field') AS TEXT) from some_table
/// ```
///
/// - For some datatypes you'll need to do some manual serialization/deserialization. For example,
/// suppose one of your json fields is an array of digests. Then you'll need to
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
///     the `parse_json_array` function:
///
///     ```ignore
///         let some_array = parse_json_array(some_array_field)
///         .into_iter()
///         .map(parse_json_byte_str)
///         .collect::<Result<Vec<u8>, _>>()?;
///     ```
/// - Thus, if needed you can create a struct representing the json values and use serde_json to
/// simplify all of the serialization/deserialization logic
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
        committed_notes: SyncedNewNotes,
        committed_transactions: &[TransactionId],
        new_mmr_peaks: MmrPeaks,
        new_authentication_nodes: &[(InOrderIndex, Digest)],
        updated_onchain_accounts: &[Account],
    ) -> Result<(), StoreError> {
        self.apply_state_sync(
            block_header,
            nullifiers,
            committed_notes,
            committed_transactions,
            new_mmr_peaks,
            new_authentication_nodes,
            updated_onchain_accounts,
        )
    }

    fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        self.get_transactions(transaction_filter)
    }

    fn apply_transaction(&mut self, tx_result: TransactionResult) -> Result<(), StoreError> {
        self.apply_transaction(tx_result)
    }

    fn get_input_notes(&self, note_filter: NoteFilter) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.get_input_notes(note_filter)
    }

    fn get_output_notes(
        &self,
        note_filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        self.get_output_notes(note_filter)
    }

    fn insert_input_note(&mut self, note: &InputNoteRecord) -> Result<(), StoreError> {
        self.insert_input_note(note)
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
        account_seed: Option<Word>,
        auth_info: &AuthInfo,
    ) -> Result<(), StoreError> {
        self.insert_account(account, account_seed, auth_info)
    }

    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        self.get_account_ids()
    }

    fn get_account_stubs(&self) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError> {
        self.get_account_stubs()
    }

    fn get_account_stub(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError> {
        self.get_account_stub(account_id)
    }

    fn get_account(&self, account_id: AccountId) -> Result<(Account, Option<Word>), StoreError> {
        self.get_account(account_id)
    }

    fn get_account_auth(&self, account_id: AccountId) -> Result<AuthInfo, StoreError> {
        self.get_account_auth(account_id)
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use std::env::temp_dir;

    use rusqlite::Connection;
    use uuid::Uuid;

    use super::{migrations, SqliteStore};
    use crate::{
        client::get_random_coin,
        config::{ClientConfig, RpcConfig},
        mock::{MockClient, MockRpcApi},
    };

    pub fn create_test_client() -> MockClient {
        let client_config = ClientConfig {
            store: create_test_store_path()
                .into_os_string()
                .into_string()
                .unwrap()
                .try_into()
                .unwrap(),
            rpc: RpcConfig::default(),
        };

        let rpc_endpoint = client_config.rpc.endpoint.to_string();
        let store = SqliteStore::new((&client_config).into()).unwrap();
        let rng = get_random_coin();
        let executor_store = SqliteStore::new((&client_config).into()).unwrap();

        MockClient::new(MockRpcApi::new(&rpc_endpoint), rng, store, executor_store).unwrap()
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
