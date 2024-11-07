use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use std::{path::Path, string::ToString};

use deadpool_sqlite::{Config, Hook, HookError, Pool, Runtime};
use miden_objects::{
    accounts::{Account, AccountHeader, AccountId, AuthSecretKey},
    crypto::merkle::{InOrderIndex, MmrPeaks},
    notes::{NoteTag, Nullifier},
    BlockHeader, Digest, Word,
};
use rusqlite::{vtab::array, Connection};
use tonic::async_trait;

use self::config::SqliteStoreConfig;
use super::{
    ChainMmrNodeFilter, InputNoteRecord, NoteFilter, OutputNoteRecord, Store, TransactionFilter,
};
use crate::{
    store::StoreError,
    sync::{NoteTagRecord, StateSyncUpdate},
    transactions::{TransactionRecord, TransactionStoreUpdate},
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

/// Represents a pool of connections with an sqlite database. The pool is used to interact
/// concurrently with the underlying database in a safe and efficient manner.
///
/// Current table definitions can be found at `store.sql` migration file.
pub struct SqliteStore {
    pub(crate) pool: Pool,
}

impl SqliteStore {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Store] instantiated with the specified configuration options.
    pub async fn new(config: &SqliteStoreConfig) -> Result<Self, StoreError> {
        let database_exists = Path::new(&config.database_filepath).exists();

        let connection_cfg = Config::new(config.database_filepath.clone());
        let pool = connection_cfg
            .builder(Runtime::Tokio1)
            .map_err(|err| StoreError::DatabaseError(err.to_string()))?
            .post_create(Hook::async_fn(move |conn, _| {
                Box::pin(async move {
                    // Feature used to support `IN` and `NOT IN` queries. We need to load this
                    // module for every connection we create to the DB to
                    // support the queries we want to run
                    let _ = conn
                        .interact(|conn| array::load_module(conn))
                        .await
                        .map_err(|_| HookError::message("Loading rarray module failed"))?;

                    Ok(())
                })
            }))
            .build()
            .map_err(|err| StoreError::DatabaseError(err.to_string()))?;

        if !database_exists {
            pool.get()
                .await
                .map_err(|err| StoreError::DatabaseError(err.to_string()))?
                .interact(|conn| conn.execute_batch(include_str!("store.sql")))
                .await
                .map_err(|err| StoreError::DatabaseError(err.to_string()))??;
        }

        Ok(Self { pool })
    }

    /// Interacts with the database by executing the provided function on a connection from the
    /// pool.
    ///
    /// This function is a helper method which simplifies the process of making queries to the
    /// database. It acquires a connection from the pool and executes the provided function,
    /// returning the result.
    async fn interact_with_connection<F, R>(&self, f: F) -> Result<R, StoreError>
    where
        F: FnOnce(&mut Connection) -> Result<R, StoreError> + Send + 'static,
        R: Send + 'static,
    {
        self.pool
            .get()
            .await
            .map_err(|err| StoreError::DatabaseError(err.to_string()))?
            .interact(f)
            .await
            .map_err(|err| StoreError::DatabaseError(err.to_string()))?
    }
}

// SQLite implementation of the Store trait
//
// To simplify, all implementations rely on inner SqliteStore functions that map 1:1 by name
// This way, the actual implementations are grouped by entity types in their own sub-modules
#[async_trait(?Send)]
impl Store for SqliteStore {
    async fn get_note_tags(&self) -> Result<Vec<NoteTagRecord>, StoreError> {
        self.interact_with_connection(SqliteStore::get_note_tags).await
    }

    async fn get_unique_note_tags(&self) -> Result<BTreeSet<NoteTag>, StoreError> {
        self.interact_with_connection(SqliteStore::get_unique_note_tags).await
    }

    async fn add_note_tag(&self, tag: NoteTagRecord) -> Result<bool, StoreError> {
        self.interact_with_connection(move |conn| SqliteStore::add_note_tag(conn, tag))
            .await
    }

    async fn remove_note_tag(&self, tag: NoteTagRecord) -> Result<usize, StoreError> {
        self.interact_with_connection(move |conn| SqliteStore::remove_note_tag(conn, tag))
            .await
    }

    async fn get_sync_height(&self) -> Result<u32, StoreError> {
        self.interact_with_connection(SqliteStore::get_sync_height).await
    }

    async fn apply_state_sync(&self, state_sync_update: StateSyncUpdate) -> Result<(), StoreError> {
        self.interact_with_connection(move |conn| {
            SqliteStore::apply_state_sync(conn, state_sync_update)
        })
        .await
    }

    async fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        self.interact_with_connection(move |conn| {
            SqliteStore::get_transactions(conn, transaction_filter)
        })
        .await
    }

    async fn apply_transaction(&self, tx_update: TransactionStoreUpdate) -> Result<(), StoreError> {
        self.interact_with_connection(move |conn| SqliteStore::apply_transaction(conn, tx_update))
            .await
    }

    async fn get_input_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.interact_with_connection(move |conn| SqliteStore::get_input_notes(conn, filter))
            .await
    }

    async fn get_output_notes(
        &self,
        note_filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        self.interact_with_connection(move |conn| SqliteStore::get_output_notes(conn, note_filter))
            .await
    }

    async fn upsert_input_notes(&self, notes: &[InputNoteRecord]) -> Result<(), StoreError> {
        let notes = notes.to_vec();
        self.interact_with_connection(move |conn| SqliteStore::upsert_input_notes(conn, &notes))
            .await
    }

    async fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        self.interact_with_connection(move |conn| {
            SqliteStore::insert_block_header(conn, block_header, chain_mmr_peaks, has_client_notes)
        })
        .await
    }

    async fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        let block_numbers = block_numbers.to_vec();
        self.interact_with_connection(move |conn| {
            SqliteStore::get_block_headers(conn, &block_numbers)
        })
        .await
    }

    async fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        self.interact_with_connection(SqliteStore::get_tracked_block_headers).await
    }

    async fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        self.interact_with_connection(move |conn| SqliteStore::get_chain_mmr_nodes(conn, filter))
            .await
    }

    async fn insert_chain_mmr_nodes(
        &self,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        let nodes = nodes.to_vec();
        self.interact_with_connection(move |conn| SqliteStore::insert_chain_mmr_nodes(conn, &nodes))
            .await
    }

    async fn get_chain_mmr_peaks_by_block_num(
        &self,
        block_num: u32,
    ) -> Result<MmrPeaks, StoreError> {
        self.interact_with_connection(move |conn| {
            SqliteStore::get_chain_mmr_peaks_by_block_num(conn, block_num)
        })
        .await
    }

    async fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), StoreError> {
        let account = account.clone();
        let auth_info = auth_info.clone();

        self.interact_with_connection(move |conn| {
            SqliteStore::insert_account(conn, &account, account_seed, &auth_info)
        })
        .await
    }

    async fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        self.interact_with_connection(SqliteStore::get_account_ids).await
    }

    async fn get_account_headers(&self) -> Result<Vec<(AccountHeader, Option<Word>)>, StoreError> {
        self.interact_with_connection(SqliteStore::get_account_headers).await
    }

    async fn get_account_auth_by_pub_key(
        &self,
        pub_key: Word,
    ) -> Result<AuthSecretKey, StoreError> {
        self.interact_with_connection(move |conn| {
            SqliteStore::get_account_auth_by_pub_key(conn, pub_key)
        })
        .await
    }

    async fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountHeader, Option<Word>), StoreError> {
        self.interact_with_connection(move |conn| SqliteStore::get_account_header(conn, account_id))
            .await
    }

    async fn get_account_header_by_hash(
        &self,
        account_hash: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        self.interact_with_connection(move |conn| {
            SqliteStore::get_account_header_by_hash(conn, account_hash)
        })
        .await
    }

    async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), StoreError> {
        self.interact_with_connection(move |conn| SqliteStore::get_account(conn, account_id))
            .await
    }

    async fn get_account_auth(&self, account_id: AccountId) -> Result<AuthSecretKey, StoreError> {
        self.interact_with_connection(move |conn| SqliteStore::get_account_auth(conn, account_id))
            .await
    }

    async fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        self.interact_with_connection(SqliteStore::get_unspent_input_note_nullifiers)
            .await
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use std::string::ToString;

    use super::{config::SqliteStoreConfig, SqliteStore};
    use crate::mock::create_test_store_path;

    pub(crate) async fn create_test_store() -> SqliteStore {
        let temp_file = create_test_store_path();

        SqliteStore::new(&SqliteStoreConfig {
            database_filepath: temp_file.to_string_lossy().to_string(),
        })
        .await
        .unwrap()
    }
}
