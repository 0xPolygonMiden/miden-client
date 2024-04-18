use alloc::collections::BTreeMap;

use clap::error::Result;
use miden_objects::{
    accounts::{Account, AccountId, AccountStub},
    crypto::{
        dsa::rpo_falcon512::SecretKey,
        merkle::{InOrderIndex, MmrPeaks},
    },
    notes::{NoteId, Nullifier},
    transaction::TransactionId,
    BlockHeader, Digest, Felt, Word,
};
use miden_tx::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};

use crate::{
    client::{
        sync::SyncedNewNotes,
        transactions::{TransactionRecord, TransactionResult},
    },
    errors::StoreError,
};

pub mod data_store;
pub mod sqlite_store;

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
pub trait Store {
    // TRANSACTIONS
    // --------------------------------------------------------------------------------------------

    /// Retrieves stored transactions, filtered by [TransactionFilter].
    fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError>;

    /// Applies a transaction, atomically updating the current state based on the
    /// [TransactionResult]
    ///
    /// An update involves:
    /// - Applying the resulting [AccountDelta](miden_objects::accounts::AccountDelta) and storing the new [Account] state
    /// - Storing new notes as a result of the transaction execution
    /// - Inserting the transaction into the store to track
    fn apply_transaction(&mut self, tx_result: TransactionResult) -> Result<(), StoreError>;

    // NOTES
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes from the store
    ///
    /// # Errors
    ///
    /// Returns a [StoreError::NoteNotFound] if the filter is [NoteFilter::Unique] and there is no Note with the provided ID
    fn get_input_notes(&self, filter: NoteFilter) -> Result<Vec<InputNoteRecord>, StoreError>;

    /// Retrieves the output notes from the store
    ///
    /// # Errors
    ///
    /// Returns a [StoreError::NoteNotFound] if the filter is [NoteFilter::Unique] and there is no Note with the provided ID
    fn get_output_notes(&self, filter: NoteFilter) -> Result<Vec<OutputNoteRecord>, StoreError>;

    /// Returns the nullifiers of all unspent input notes
    ///
    /// The default implementation of this method uses [Store::get_input_notes].
    fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        let nullifiers = self
            .get_input_notes(NoteFilter::Committed)?
            .iter()
            .map(|input_note| Ok(Nullifier::from(Digest::try_from(input_note.nullifier())?)))
            .collect::<Result<Vec<_>, _>>();

        nullifiers
    }

    /// Inserts the provided input note into the database
    fn insert_input_note(&mut self, note: &InputNoteRecord) -> Result<(), StoreError>;

    // CHAIN DATA
    // --------------------------------------------------------------------------------------------

    /// Retrieves a vector of [BlockHeader]s filtered by the provided block numbers.
    ///
    /// The returned vector may not contain some or all of the requested block headers. It's up to
    /// the callee to check whether all requested block headers were found.
    ///
    /// For each block header an additional boolean value is returned representing whether the block
    /// contains notes relevant to the client.
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
    fn get_block_header_by_num(
        &self,
        block_number: u32,
    ) -> Result<(BlockHeader, bool), StoreError> {
        self.get_block_headers(&[block_number])
            .map(|block_headers_list| block_headers_list.first().cloned())
            .and_then(|block_header| {
                block_header.ok_or(StoreError::BlockHeaderNotFound(block_number))
            })
    }

    /// Retrieves a list of [BlockHeader] that include relevant notes to the client.
    fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError>;

    /// Retrieves all MMR authentication nodes based on [ChainMmrNodeFilter].
    fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError>;

    /// Returns peaks information from the blockchain by a specific block number.
    ///
    /// If there is no chain MMR info stored for the provided block returns an empty [MmrPeaks]
    fn get_chain_mmr_peaks_by_block_num(&self, block_num: u32) -> Result<MmrPeaks, StoreError>;

    /// Inserts a block header into the store, alongside peaks information at the block's height.
    ///
    /// `has_client_notes` describes whether the block has relevant notes to the client; this means
    /// the client might want to authenticate merkle paths based on this value.
    fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError>;

    // ACCOUNT
    // --------------------------------------------------------------------------------------------

    /// Returns the account IDs of all accounts stored in the database
    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError>;

    /// Returns a list of [AccountStub] of all accounts stored in the database along with the seeds
    /// used to create them.
    ///
    /// Said accounts' state is the state after the last performed sync.
    fn get_account_stubs(&self) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError>;

    /// Retrieves an [AccountStub] object for the specified [AccountId] along with the seed
    /// used to create it. The seed will be returned if the account is new, otherwise it
    /// will be `None`.
    ///
    /// Said account's state is the state according to the last sync performed.
    ///
    /// # Errors
    /// Returns a `StoreError::AccountDataNotFound` if there is no account for the provided ID
    fn get_account_stub(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError>;

    /// Retrieves a full [Account] object. The seed will be returned if the account is new,
    /// otherwise it will be `None`.
    ///
    /// This function returns the [Account]'s latest state. If the account is new (that is, has
    /// never executed a trasaction), the returned seed will be `Some(Word)`; otherwise the seed
    /// will be `None`
    ///
    /// # Errors
    ///
    /// Returns a `StoreError::AccountDataNotFound` if there is no account for the provided ID
    fn get_account(&self, account_id: AccountId) -> Result<(Account, Option<Word>), StoreError>;

    /// Retrieves an account's [AuthInfo], utilized to authenticate the account.
    ///
    /// # Errors
    ///
    /// Returns a `StoreError::AccountDataNotFound` if there is no account for the provided ID
    fn get_account_auth(&self, account_id: AccountId) -> Result<AuthInfo, StoreError>;

    /// Inserts an [Account] along with the seed used to create it and its [AuthInfo]
    fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthInfo,
    ) -> Result<(), StoreError>;

    // SYNC
    // --------------------------------------------------------------------------------------------

    /// Returns the note tags that the client is interested in.
    fn get_note_tags(&self) -> Result<Vec<u64>, StoreError>;

    /// Adds a note tag to the list of tags that the client is interested in.
    fn add_note_tag(&mut self, tag: u64) -> Result<bool, StoreError>;

    /// Returns the block number of the last state sync block.
    fn get_sync_height(&self) -> Result<u32, StoreError>;

    /// Applies the state sync update to the store. An update involves:
    ///
    /// - Inserting the new block header to the store alongside new MMR peaks information
    /// - Updating the notes, marking them as `committed` or `consumed` based on incoming
    ///   inclusion proofs and nullifiers
    /// - Updating transactions in the store, marking as `committed` the ones provided with
    /// `committed_transactions`
    /// - Storing new MMR authentication nodes
    fn apply_state_sync(
        &mut self,
        block_header: BlockHeader,
        nullifiers: Vec<Digest>,
        new_note_details: SyncedNewNotes,
        committed_transactions: &[TransactionId],
        new_mmr_peaks: MmrPeaks,
        new_authentication_nodes: &[(InOrderIndex, Digest)],
        updated_onchain_accounts: &[Account],
    ) -> Result<(), StoreError>;
}

// DATABASE AUTH INFO
// ================================================================================================

/// Represents the types of authentication information of accounts
#[derive(Debug)]
pub enum AuthInfo {
    RpoFalcon512(SecretKey),
}

const RPO_FALCON512_AUTH: u8 = 0;

impl AuthInfo {
    /// Returns byte identifier of specific AuthInfo
    const fn type_byte(&self) -> u8 {
        match self {
            AuthInfo::RpoFalcon512(_) => RPO_FALCON512_AUTH,
        }
    }

    /// Returns the authentication information as a tuple of (key, value)
    /// that can be input to the advice map at the moment of transaction execution.
    pub fn into_advice_inputs(self) -> (Word, Vec<Felt>) {
        match self {
            AuthInfo::RpoFalcon512(key) => {
                let pub_key: Word = key.public_key().into();
                let mut pk_sk_bytes = key.to_bytes();
                pk_sk_bytes.append(&mut pub_key.to_bytes());

                (pub_key, pk_sk_bytes.iter().map(|a| Felt::new(*a as u64)).collect::<Vec<Felt>>())
            },
        }
    }
}

impl Serializable for AuthInfo {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let mut bytes = vec![self.type_byte()];
        match self {
            AuthInfo::RpoFalcon512(key_pair) => {
                bytes.append(&mut key_pair.to_bytes());
                target.write_bytes(&bytes);
            },
        }
    }
}

impl Deserializable for AuthInfo {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let auth_type: u8 = source.read_u8()?;
        match auth_type {
            RPO_FALCON512_AUTH => {
                let key_pair = SecretKey::read_from(source)?;
                Ok(AuthInfo::RpoFalcon512(key_pair))
            },
            val => Err(DeserializationError::InvalidValue(val.to_string())),
        }
    }
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

pub enum NoteFilter {
    /// Return a list of all notes ([InputNoteRecord] or [OutputNoteRecord]).
    All,
    /// Filter by consumed notes ([InputNoteRecord] or [OutputNoteRecord]). notes that have been used as inputs in transactions.
    Consumed,
    /// Return a list of committed notes ([InputNoteRecord] or [OutputNoteRecord]). These represent notes that the blockchain
    /// has included in a block, and for which we are storing anchor data.
    Committed,
    /// Return a list of pending notes ([InputNoteRecord] or [OutputNoteRecord]). These represent notes for which the store
    /// does not have anchor data.
    Pending,
    /// Return a list containing the note that matches with the provided NoteId.
    Unique(NoteId),
}
