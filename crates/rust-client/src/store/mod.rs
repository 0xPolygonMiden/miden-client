//! Defines the storage interfaces used by the Miden client.
//!
//! It provides mechanisms for persisting and retrieving data, such as account states, transaction
//! history, block headers, notes, and MMR nodes.
//!
//! ## Overview
//!
//! The storage module is central to the Miden client’s persistence layer. It defines the
//! [`Store`] trait which abstracts over any concrete storage implementation. The trait exposes
//! methods to (among others):
//!
//! - Retrieve and update transactions, notes, and accounts.
//! - Store and query block headers along with MMR peaks and authentication nodes.
//! - Manage note tags for synchronizing with the node.
//!
//! These are all used by the Miden client to provide transaction execution in the correct contexts.
//!
//! In addition to the main [`Store`] trait, the module provides types for filtering queries, such
//! as [`TransactionFilter`] and [`NoteFilter`], to narrow down the set of returned transactions or
//! notes. For more advanced usage, see the documentation of individual methods in the [`Store`]
//! trait.

use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::fmt::Debug;

use async_trait::async_trait;
use miden_objects::{
    Digest, Word,
    account::{Account, AccountCode, AccountHeader, AccountId},
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InOrderIndex, MmrPeaks},
    note::{NoteId, NoteTag, Nullifier},
    transaction::TransactionId,
};

use crate::{
    note::NoteUpdates,
    sync::{NoteTagRecord, StateSyncUpdate},
    transaction::{TransactionRecord, TransactionStoreUpdate},
};

/// Contains [`ClientDataStore`] to automatically implement [`DataStore`] for anything that
/// implements [`Store`]. This isn't public because it's an implementation detail to instantiate the
/// executor.
///
/// The user is tasked with creating a [`Store`] which the client will wrap into a
/// [`ClientDataStore`] at creation time.
pub(crate) mod data_store;

mod errors;
pub use errors::*;

#[cfg(all(feature = "sqlite", feature = "idxdb"))]
compile_error!("features `sqlite` and `idxdb` are mutually exclusive");

#[cfg(feature = "sqlite")]
pub mod sqlite_store;

#[cfg(feature = "idxdb")]
pub mod web_store;

mod account;
pub use account::{AccountRecord, AccountStatus, AccountUpdates};
mod note_record;
pub use note_record::{
    InputNoteRecord, InputNoteState, NoteExportType, NoteRecordError, OutputNoteRecord,
    OutputNoteState, input_note_states,
};

// STORE TRAIT
// ================================================================================================

/// The [`Store`] trait exposes all methods that the client store needs in order to track the
/// current state.
///
/// All update functions are implied to be atomic. That is, if multiple entities are meant to be
/// updated as part of any single function and an error is returned during its execution, any
/// changes that might have happened up to that point need to be rolled back and discarded.
///
/// Because the [`Store`]'s ownership is shared between the executor and the client, interior
/// mutability is expected to be implemented, which is why all methods receive `&self` and
/// not `&mut self`.
#[async_trait(?Send)]
pub trait Store: Send + Sync {
    /// Returns the current timestamp tracked by the store, measured in non-leap seconds since
    /// Unix epoch. If the store implementation is incapable of tracking time, it should return
    /// `None`.
    ///
    /// This method is used to add time metadata to notes' states. This information doesn't have a
    /// functional impact on the client's operation, it's shown to the user for informational
    /// purposes.
    fn get_current_timestamp(&self) -> Option<u64>;

    // TRANSACTIONS
    // --------------------------------------------------------------------------------------------

    /// Retrieves stored transactions, filtered by [`TransactionFilter`].
    async fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError>;

    /// Applies a transaction, atomically updating the current state based on the
    /// [`TransactionStoreUpdate`].
    ///
    /// An update involves:
    /// - Updating the stored account which is being modified by the transaction.
    /// - Storing new input/output notes and payback note details as a result of the transaction
    ///   execution.
    /// - Updating the input notes that are being processed by the transaction.
    /// - Inserting the new tracked tags into the store.
    /// - Inserting the transaction into the store to track.
    async fn apply_transaction(&self, tx_update: TransactionStoreUpdate) -> Result<(), StoreError>;

    // NOTES
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes from the store.
    async fn get_input_notes(&self, filter: NoteFilter)
    -> Result<Vec<InputNoteRecord>, StoreError>;

    /// Retrieves the output notes from the store.
    async fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError>;

    /// Returns the nullifiers of all unspent input notes.
    ///
    /// The default implementation of this method uses [Store::get_input_notes].
    async fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        self.get_input_notes(NoteFilter::Unspent)
            .await?
            .iter()
            .map(|input_note| Ok(input_note.nullifier()))
            .collect::<Result<Vec<_>, _>>()
    }

    /// Inserts the provided input notes into the database. If a note with the same ID already
    /// exists, it will be replaced.
    async fn upsert_input_notes(&self, notes: &[InputNoteRecord]) -> Result<(), StoreError>;

    // CHAIN DATA
    // --------------------------------------------------------------------------------------------

    /// Retrieves a vector of [`BlockHeader`]s filtered by the provided block numbers.
    ///
    /// The returned vector may not contain some or all of the requested block headers. It's up to
    /// the callee to check whether all requested block headers were found.
    ///
    /// For each block header an additional boolean value is returned representing whether the block
    /// contains notes relevant to the client.
    async fn get_block_headers(
        &self,
        block_numbers: &BTreeSet<BlockNumber>,
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError>;

    /// Retrieves a [`BlockHeader`] corresponding to the provided block number and a boolean value
    /// that represents whether the block contains notes relevant to the client. Returns `None` if
    /// the block is not found.
    ///
    /// The default implementation of this method uses [Store::get_block_headers].
    async fn get_block_header_by_num(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<(BlockHeader, bool)>, StoreError> {
        self.get_block_headers(&[block_number].into_iter().collect())
            .await
            .map(|mut block_headers_list| block_headers_list.pop())
    }

    /// Retrieves a list of [`BlockHeader`] that include relevant notes to the client.
    async fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError>;

    /// Retrieves all MMR authentication nodes based on [PartialBlockchainFilter].
    async fn get_partial_blockchain_nodes(
        &self,
        filter: PartialBlockchainFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError>;

    /// Inserts blockchain MMR authentication nodes.
    ///
    /// In the case where the [`InOrderIndex`] already exists on the table, the insertion is
    /// ignored.
    async fn insert_partial_blockchain_nodes(
        &self,
        nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError>;

    /// Returns peaks information from the blockchain by a specific block number.
    ///
    /// If there is no partial blockchain info stored for the provided block returns an empty
    /// [`MmrPeaks`].
    async fn get_partial_blockchain_peaks_by_block_num(
        &self,
        block_num: BlockNumber,
    ) -> Result<MmrPeaks, StoreError>;

    /// Inserts a block header into the store, alongside peaks information at the block's height.
    ///
    /// `has_client_notes` describes whether the block has relevant notes to the client; this means
    /// the client might want to authenticate merkle paths based on this value.
    /// If the block header exists and `has_client_notes` is `true` then the `has_client_notes`
    /// column is updated to `true` to signify that the block now contains a relevant note.
    async fn insert_block_header(
        &self,
        block_header: &BlockHeader,
        blockchain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError>;

    // ACCOUNT
    // --------------------------------------------------------------------------------------------

    /// Returns the account IDs of all accounts stored in the database.
    async fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError>;

    /// Returns a list of [`AccountHeader`] of all accounts stored in the database along with their
    /// statuses.
    ///
    /// Said accounts' state is the state after the last performed sync.
    async fn get_account_headers(&self) -> Result<Vec<(AccountHeader, AccountStatus)>, StoreError>;

    /// Retrieves an [`AccountHeader`] object for the specified [`AccountId`] along with its status.
    /// Returns `None` if the account is not found.
    ///
    /// Said account's state is the state according to the last sync performed.
    async fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<Option<(AccountHeader, AccountStatus)>, StoreError>;

    /// Returns an [`AccountHeader`] corresponding to the stored account state that matches the
    /// given commitment. If no account state matches the provided commitment, `None` is returned.
    async fn get_account_header_by_commitment(
        &self,
        account_commitment: Digest,
    ) -> Result<Option<AccountHeader>, StoreError>;

    /// Retrieves a full [`AccountRecord`] object, this contains the account's latest state along
    /// with its status. Returns `None` if the account is not found.
    async fn get_account(&self, account_id: AccountId)
    -> Result<Option<AccountRecord>, StoreError>;

    /// Inserts an [`Account`] along with the seed used to create it.
    async fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
    ) -> Result<(), StoreError>;

    /// Upserts the account code for a foreign account. This value will be used as a cache of known
    /// script roots and added to the `GetForeignAccountCode` request.
    async fn upsert_foreign_account_code(
        &self,
        account_id: AccountId,
        code: AccountCode,
    ) -> Result<(), StoreError>;

    /// Retrieves the cached account code for various foreign accounts.
    async fn get_foreign_account_code(
        &self,
        account_ids: Vec<AccountId>,
    ) -> Result<BTreeMap<AccountId, AccountCode>, StoreError>;

    /// Updates an existing [`Account`] with a new state.
    ///
    /// # Errors
    ///
    /// Returns a `StoreError::AccountDataNotFound` if there is no account for the provided ID.
    async fn update_account(&self, new_account_state: &Account) -> Result<(), StoreError>;

    // SYNC
    // --------------------------------------------------------------------------------------------

    /// Returns the note tag records that the client is interested in.
    async fn get_note_tags(&self) -> Result<Vec<NoteTagRecord>, StoreError>;

    /// Returns the unique note tags (without source) that the client is interested in.
    async fn get_unique_note_tags(&self) -> Result<BTreeSet<NoteTag>, StoreError> {
        Ok(self.get_note_tags().await?.into_iter().map(|r| r.tag).collect())
    }

    /// Adds a note tag to the list of tags that the client is interested in.
    ///
    /// If the tag was already being tracked, returns false since no new tags were actually added.
    /// Otherwise true.
    async fn add_note_tag(&self, tag: NoteTagRecord) -> Result<bool, StoreError>;

    /// Removes a note tag from the list of tags that the client is interested in.
    ///
    /// If the tag wasn't present in the store returns false since no tag was actually removed.
    /// Otherwise returns true.
    async fn remove_note_tag(&self, tag: NoteTagRecord) -> Result<usize, StoreError>;

    /// Returns the block number of the last state sync block.
    async fn get_sync_height(&self) -> Result<BlockNumber, StoreError>;

    /// Applies the state sync update to the store. An update involves:
    ///
    /// - Inserting the new block header to the store alongside new MMR peaks information.
    /// - Updating the corresponding tracked input/output notes.
    /// - Removing note tags that are no longer relevant.
    /// - Updating transactions in the store, marking as `committed` or `discarded`.
    /// - Storing new MMR authentication nodes.
    /// - Updating the tracked on-chain accounts.
    async fn apply_state_sync(&self, state_sync_update: StateSyncUpdate) -> Result<(), StoreError>;

    /// Applies nullifier updates to database.
    /// Nullifiers are retrieved after completing a `StateSync`.
    ///
    /// This operation is temporary, to be removed as part of miden-client/650.
    async fn apply_nullifiers(
        &self,
        note_updates: NoteUpdates,
        transactions_to_discard: Vec<TransactionId>,
    ) -> Result<(), StoreError>;
}

// PARTIAL BLOCKCHAIN NODE FILTER
// ================================================================================================
/// Filters for searching specific MMR nodes.
// TODO: Should there be filters for specific blocks instead of nodes?
pub enum PartialBlockchainFilter {
    /// Return all nodes.
    All,
    /// Filter by the specified in-order indices.
    List(Vec<InOrderIndex>),
}

// TRANSACTION FILTERS
// ================================================================================================

/// Filters for narrowing the set of transactions returned by the client's store.
#[derive(Debug, Clone)]
pub enum TransactionFilter {
    /// Return all transactions.
    All,
    /// Filter by transactions that haven't yet been committed to the blockchain as per the last
    /// sync.
    Uncomitted,
    /// Return a list of the transaction that matches the provided [`TransactionId`]s.
    Ids(Vec<TransactionId>),
    /// Return a list of the expired transactions that were executed before the provided
    /// [`BlockNumber`]. Transactions created after the provided block number are not
    /// considered.
    ///
    /// A transaction is considered expired if is uncommitted and the transaction's block number
    /// is less than the provided block number.
    ExpiredBefore(BlockNumber),
}

// NOTE FILTER
// ================================================================================================

/// Filters for narrowing the set of notes returned by the client's store.
#[derive(Debug, Clone)]
pub enum NoteFilter {
    /// Return a list of all notes ([`InputNoteRecord`] or [`OutputNoteRecord`]).
    All,
    /// Return a list of committed notes ([`InputNoteRecord`] or [`OutputNoteRecord`]). These
    /// represent notes that the blockchain has included in a block, and for which we are
    /// storing anchor data.
    Committed,
    /// Filter by consumed notes ([`InputNoteRecord`] or [`OutputNoteRecord`]). notes that have
    /// been used as inputs in transactions.
    Consumed,
    /// Return a list of expected notes ([`InputNoteRecord`] or [`OutputNoteRecord`]). These
    /// represent notes for which the store doesn't have anchor data.
    Expected,
    /// Return a list containing any notes that match with the provided [`NoteId`] vector.
    List(Vec<NoteId>),
    /// Return a list containing any notes that match the provided [`Nullifier`] vector.
    Nullifiers(Vec<Nullifier>),
    /// Return a list of notes that are currently being processed. This filter doesn't apply to
    /// output notes.
    Processing,
    /// Return a list containing the note that matches with the provided [`NoteId`]. The query will
    /// return an error if the note isn't found.
    Unique(NoteId),
    /// Return a list containing notes that haven't been nullified yet, this includes expected,
    /// committed, processing and unverified notes.
    Unspent,
    /// Return a list containing notes with unverified inclusion proofs. This filter doesn't apply
    /// to output notes.
    Unverified,
}
