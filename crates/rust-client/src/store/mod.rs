use alloc::collections::BTreeMap;
use core::fmt::Debug;

use miden_objects::{
    accounts::{Account, AccountId, AccountStub, AuthSecretKey},
    crypto::merkle::{InOrderIndex, MmrPeaks},
    notes::{NoteId, NoteInclusionProof, NoteMetadata, NoteTag, Nullifier},
    BlockHeader, Digest, Word,
};
use winter_maybe_async::{maybe_async, maybe_await};

use crate::{
    errors::StoreError,
    sync::StateSyncUpdate,
    transactions::{TransactionRecord, TransactionResult},
};

pub mod data_store;

#[cfg(feature = "sqlite")]
pub mod sqlite_store;

#[cfg(feature = "idxdb")]
pub mod web_store;

mod note_record;
pub use note_record::{InputNoteRecord, NoteRecordDetails, NoteStatus, OutputNoteRecord};

// STORE TRAIT
// ================================================================================================

/// The [Store] trait exposes all methods that the client store needs in order to track the current
/// state.
///
/// All update functions are implied to be atomic. That is, if multiple entities are meant to be
/// updated as part of any single function and an error is returned during its execution, any
/// changes that might have happened up to that point need to be rolled back and discarded.
///
/// Because the [Store]'s ownership is shared between the executor and the client, interior
/// mutability is expected to be implemented, which is why all methods receive `&self` and
/// not `&mut self`.
pub trait Store {
    // TRANSACTIONS
    // --------------------------------------------------------------------------------------------

    /// Retrieves stored transactions, filtered by [TransactionFilter].
    #[maybe_async]
    fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError>;

    /// Applies a transaction, atomically updating the current state based on the
    /// [TransactionResult]
    ///
    /// An update involves:
    /// - Applying the resulting [AccountDelta](miden_objects::accounts::AccountDelta) and storing the new [Account] state
    /// - Storing new notes and payback note details as a result of the transaction execution
    /// - Inserting the transaction into the store to track
    #[maybe_async]
    fn apply_transaction(&self, tx_result: TransactionResult) -> Result<(), StoreError>;

    // NOTES
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes from the store
    ///
    /// # Errors
    ///
    /// Returns a [StoreError::NoteNotFound] if the filter is [NoteFilter::Unique] and there is no Note with the provided ID
    #[maybe_async]
    fn get_input_notes(&self, filter: NoteFilter) -> Result<Vec<InputNoteRecord>, StoreError>;

    /// Retrieves the output notes from the store
    ///
    /// # Errors
    ///
    /// Returns a [StoreError::NoteNotFound] if the filter is [NoteFilter::Unique] and there is no Note with the provided ID
    #[maybe_async]
    fn get_output_notes(&self, filter: NoteFilter) -> Result<Vec<OutputNoteRecord>, StoreError>;

    /// Returns the nullifiers of all unspent input notes
    ///
    /// The default implementation of this method uses [Store::get_input_notes].
    #[maybe_async]
    fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        let nullifiers = maybe_await!(self.get_input_notes(NoteFilter::Committed))?
            .iter()
            .chain(maybe_await!(self.get_input_notes(NoteFilter::Processing))?.iter())
            .map(|input_note| Ok(Nullifier::from(Digest::try_from(input_note.nullifier())?)))
            .collect::<Result<Vec<_>, _>>();

        nullifiers
    }

    /// Returns the committed notes that don't have their block header tracked
    ///
    /// The default implementation of this method uses [Store::get_tracked_block_headers] and [Store::get_input_notes].
    #[maybe_async]
    fn get_notes_without_block_header(&self) -> Result<Vec<InputNoteRecord>, StoreError> {
        let tracked_block_nums: Vec<u32> = maybe_await!(self.get_tracked_block_headers())?
            .iter()
            .map(|header| header.block_num())
            .collect();
        let notes_without_block_header: Vec<InputNoteRecord> =
            maybe_await!(self.get_input_notes(NoteFilter::Committed))?
                .into_iter()
                .filter(|note| {
                    !tracked_block_nums.contains(
                        &note
                            .inclusion_proof()
                            .expect("Committed note should have inclusion proof")
                            .origin()
                            .block_num,
                    )
                })
                .collect();
        Ok(notes_without_block_header)
    }

    /// Inserts the provided input note into the database
    #[maybe_async]
    fn insert_input_note(&self, note: InputNoteRecord) -> Result<(), StoreError>;

    #[maybe_async]
    /// Updates the inclusion proof of the input note with the provided ID
    fn update_note_inclusion_proof(
        &self,
        note_id: NoteId,
        inclusion_proof: NoteInclusionProof,
    ) -> Result<(), StoreError>;

    #[maybe_async]
    /// Updates the metadata of the input note with the provided ID
    fn update_note_metadata(
        &self,
        note_id: NoteId,
        metadata: NoteMetadata,
    ) -> Result<(), StoreError>;

    // CHAIN DATA
    // --------------------------------------------------------------------------------------------

    /// Retrieves a vector of [BlockHeader]s filtered by the provided block numbers.
    ///
    /// The returned vector may not contain some or all of the requested block headers. It's up to
    /// the callee to check whether all requested block headers were found.
    ///
    /// For each block header an additional boolean value is returned representing whether the block
    /// contains notes relevant to the client.
    #[maybe_async]
    fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError>;

    /// Retrieves a [BlockHeader] corresponding to the provided block number and a boolean value
    /// that represents whether the block contains notes relevant to the client.
    ///
    /// The default implementation of this method uses [Store::get_block_headers].
    ///
    /// # Errors
    /// Returns a [StoreError::BlockHeaderNotFound] if the block was not found.
    #[maybe_async]
    fn get_block_header_by_num(
        &self,
        block_number: u32,
    ) -> Result<(BlockHeader, bool), StoreError> {
        maybe_await!(self.get_block_headers(&[block_number]))
            .map(|block_headers_list| block_headers_list.first().cloned())
            .and_then(|block_header| {
                block_header.ok_or(StoreError::BlockHeaderNotFound(block_number))
            })
    }

    /// Retrieves a list of [BlockHeader] that include relevant notes to the client.
    #[maybe_async]
    fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError>;

    /// Retrieves all MMR authentication nodes based on [ChainMmrNodeFilter].
    #[maybe_async]
    fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError>;

    /// Inserts MMR authentication nodes.
    ///
    /// In the case where the [InOrderIndex] already exists on the table, the insertion is ignored
    #[maybe_async]
    fn insert_chain_mmr_nodes(&self, nodes: &[(InOrderIndex, Digest)]) -> Result<(), StoreError>;

    /// Returns peaks information from the blockchain by a specific block number.
    ///
    /// If there is no chain MMR info stored for the provided block returns an empty [MmrPeaks]
    #[maybe_async]
    fn get_chain_mmr_peaks_by_block_num(&self, block_num: u32) -> Result<MmrPeaks, StoreError>;

    /// Inserts a block header into the store, alongside peaks information at the block's height.
    ///
    /// `has_client_notes` describes whether the block has relevant notes to the client; this means
    /// the client might want to authenticate merkle paths based on this value.
    #[maybe_async]
    fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError>;

    // ACCOUNT
    // --------------------------------------------------------------------------------------------

    /// Returns the account IDs of all accounts stored in the database
    #[maybe_async]
    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError>;

    /// Returns a list of [AccountStub] of all accounts stored in the database along with the seeds
    /// used to create them.
    ///
    /// Said accounts' state is the state after the last performed sync.
    #[maybe_async]
    fn get_account_stubs(&self) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError>;

    /// Retrieves an [AccountStub] object for the specified [AccountId] along with the seed
    /// used to create it. The seed will be returned if the account is new, otherwise it
    /// will be `None`.
    ///
    /// Said account's state is the state according to the last sync performed.
    ///
    /// # Errors
    /// Returns a `StoreError::AccountDataNotFound` if there is no account for the provided ID
    #[maybe_async]
    fn get_account_stub(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError>;

    /// Retrieves a full [Account] object. The seed will be returned if the account is new,
    /// otherwise it will be `None`.
    ///
    /// This function returns the [Account]'s latest state. If the account is new (that is, has
    /// never executed a transaction), the returned seed will be `Some(Word)`; otherwise the seed
    /// will be `None`
    ///
    /// # Errors
    ///
    /// Returns a `StoreError::AccountDataNotFound` if there is no account for the provided ID
    #[maybe_async]
    fn get_account(&self, account_id: AccountId) -> Result<(Account, Option<Word>), StoreError>;

    /// Retrieves an account's [AuthSecretKey], utilized to authenticate the account.
    ///
    /// # Errors
    ///
    /// Returns a `StoreError::AccountDataNotFound` if there is no account for the provided ID
    #[maybe_async]
    fn get_account_auth(&self, account_id: AccountId) -> Result<AuthSecretKey, StoreError>;

    /// Retrieves an account's [AuthSecretKey] by pub key, utilized to authenticate the account.
    /// This is mainly used for authentication in transactions.
    ///
    /// # Errors
    ///
    /// Returns a `StoreError::AccountKeyNotFound` if there is no account for the provided key
    fn get_account_auth_by_pub_key(&self, pub_key: Word) -> Result<AuthSecretKey, StoreError>;

    /// Inserts an [Account] along with the seed used to create it and its [AuthSecretKey]
    #[maybe_async]
    fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthSecretKey,
    ) -> Result<(), StoreError>;

    // SYNC
    // --------------------------------------------------------------------------------------------

    /// Returns the note tags that the client is interested in.
    #[maybe_async]
    fn get_note_tags(&self) -> Result<Vec<NoteTag>, StoreError>;

    /// Adds a note tag to the list of tags that the client is interested in.
    ///
    /// If the tag was already being tracked, returns false since no new tags were actually added. Otherwise true.
    #[maybe_async]
    fn add_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError>;

    /// Removes a note tag from the list of tags that the client is interested in.
    ///
    /// If the tag was not present in the store returns false since no tag was actually removed.
    /// Otherwise returns true.
    #[maybe_async]
    fn remove_note_tag(&self, tag: NoteTag) -> Result<bool, StoreError>;

    /// Returns the block number of the last state sync block.
    #[maybe_async]
    fn get_sync_height(&self) -> Result<u32, StoreError>;

    /// Applies the state sync update to the store. An update involves:
    ///
    /// - Inserting the new block header to the store alongside new MMR peaks information
    /// - Updating the notes, marking them as `committed` or `consumed` based on incoming
    ///   inclusion proofs and nullifiers
    /// - Updating transactions in the store, marking as `committed` the ones provided with
    ///   `committed_transactions`
    /// - Storing new MMR authentication nodes
    #[maybe_async]
    fn apply_state_sync(&self, state_sync_update: StateSyncUpdate) -> Result<(), StoreError>;
}

// CHAIN MMR NODE FILTER
// ================================================================================================

pub enum ChainMmrNodeFilter<'a> {
    /// Return all nodes.
    All,
    /// Filter by the specified in-order indices.
    List(&'a [InOrderIndex]),
}

// TRANSACTION FILTERS
// ================================================================================================

pub enum TransactionFilter {
    /// Return all transactions.
    All,
    /// Filter by transactions that have not yet been committed to the blockchain as per the last
    /// sync.
    Uncomitted,
}

// NOTE FILTER
// ================================================================================================

#[derive(Debug, Clone)]
pub enum NoteFilter<'a> {
    /// Return a list of all notes ([InputNoteRecord] or [OutputNoteRecord]).
    All,
    /// Filter by consumed notes ([InputNoteRecord] or [OutputNoteRecord]). notes that have been used as inputs in transactions.
    Consumed,
    /// Return a list of committed notes ([InputNoteRecord] or [OutputNoteRecord]). These represent notes that the blockchain
    /// has included in a block, and for which we are storing anchor data.
    Committed,
    /// Return a list of expected notes ([InputNoteRecord] or [OutputNoteRecord]). These represent notes for which the store
    /// does not have anchor data.
    Expected,
    /// Return a list of notes that are currently being processed.
    Processing,
    /// Return a list of notes that the client ignores in sync.
    Ignored,
    /// Return a list containing the note that matches with the provided [NoteId].
    List(&'a [NoteId]),
    /// Return a list containing the note that matches with the provided [NoteId].
    Unique(NoteId),
}
