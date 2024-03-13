use crate::{
    client::transactions::{TransactionRecord, TransactionResult},
    errors::{ClientError, StoreError},
};
use clap::error::Result;
use miden_objects::{
    accounts::{Account, AccountId, AccountStub},
    crypto::{
        dsa::rpo_falcon512::KeyPair,
        merkle::{InOrderIndex, MmrPeaks},
    },
    notes::{Note, NoteId, NoteInclusionProof, Nullifier},
    transaction::{InputNote, TransactionId},
    utils::collections::BTreeMap,
    BlockHeader, Digest, Word,
};

use miden_tx::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};
use serde::{Deserialize, Serialize};

pub mod data_store;
pub mod sqlite_store;

#[cfg(any(test, feature = "mock"))]
pub mod mock_executor_data_store;

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
    /// - Applying the resulting [AccountDelta] and storing the new [Account] state
    /// - Storing new notes as a result of the transaction execution
    /// - Inserting the transaction into the store to track
    fn apply_transaction(&mut self, tx_result: TransactionResult) -> Result<(), StoreError>;

    // NOTES
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes from the store
    fn get_input_notes(&self, filter: NoteFilter) -> Result<Vec<InputNoteRecord>, StoreError>;

    /// Retrieves the output notes from the store
    fn get_output_notes(&self, filter: NoteFilter) -> Result<Vec<InputNoteRecord>, StoreError>;

    /// Retrieves an [InputNoteRecord] for the input note corresponding to the specified ID from
    /// the store.
    ///
    /// # Errors
    ///
    /// Returns a [StoreError::InputNoteNotFound] if there is no Note with the provided ID
    fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError>;

    /// Returns the nullifiers of all unspent input notes
    ///
    /// The default implementation of this method uses [Store::get_input_notes].
    fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        let nullifiers = self
            .get_input_notes(NoteFilter::Committed)?
            .iter()
            .map(|input_note| input_note.note().nullifier())
            .collect();

        Ok(nullifiers)
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
        committed_notes: Vec<(NoteId, NoteInclusionProof)>,
        committed_transactions: &[TransactionId],
        new_mmr_peaks: MmrPeaks,
        new_authentication_nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError>;
}

// DATABASE AUTH INFO
// ================================================================================================

/// Represents the types of authentication information of accounts
#[derive(Debug)]
pub enum AuthInfo {
    RpoFalcon512(KeyPair),
}

const RPO_FALCON512_AUTH: u8 = 0;

impl AuthInfo {
    /// Returns byte identifier of specific AuthInfo
    const fn type_byte(&self) -> u8 {
        match self {
            AuthInfo::RpoFalcon512(_) => RPO_FALCON512_AUTH,
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
            }
        }
    }
}

impl Deserializable for AuthInfo {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let auth_type: u8 = source.read_u8()?;
        match auth_type {
            RPO_FALCON512_AUTH => {
                let key_pair = KeyPair::read_from(source)?;
                Ok(AuthInfo::RpoFalcon512(key_pair))
            }
            val => Err(DeserializationError::InvalidValue(val.to_string())),
        }
    }
}

// INPUT NOTE RECORD
// ================================================================================================

/// Represents a Note of which the [Store] can keep track and retrieve.
///
/// An [InputNoteRecord] contains all the information of a [Note], in addition of (optionally)
/// the [NoteInclusionProof] that identifies when the note was included in the chain. Once the
/// proof is set, the [InputNoteRecord] can be transformed into an [InputNote] and used as input
/// for transactions.
#[derive(Clone, Debug, PartialEq)]
pub struct InputNoteRecord {
    note: Note,
    inclusion_proof: Option<NoteInclusionProof>,
}

impl InputNoteRecord {
    pub fn new(note: Note, inclusion_proof: Option<NoteInclusionProof>) -> InputNoteRecord {
        InputNoteRecord {
            note,
            inclusion_proof,
        }
    }
    pub fn note(&self) -> &Note {
        &self.note
    }

    pub fn note_id(&self) -> NoteId {
        self.note.id()
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        self.inclusion_proof.as_ref()
    }
}

impl Serializable for InputNoteRecord {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.note().write_into(target);
        self.inclusion_proof.write_into(target);
    }
}

impl Deserializable for InputNoteRecord {
    fn read_from<R: ByteReader>(
        source: &mut R,
    ) -> std::prelude::v1::Result<Self, DeserializationError> {
        let note = Note::read_from(source)?;
        let proof = Option::<NoteInclusionProof>::read_from(source)?;
        Ok(InputNoteRecord::new(note, proof))
    }
}

impl From<Note> for InputNoteRecord {
    fn from(note: Note) -> Self {
        InputNoteRecord {
            note,
            inclusion_proof: None,
        }
    }
}

impl From<InputNote> for InputNoteRecord {
    fn from(recorded_note: InputNote) -> Self {
        InputNoteRecord {
            note: recorded_note.note().clone(),
            inclusion_proof: Some(recorded_note.proof().clone()),
        }
    }
}

impl TryInto<InputNote> for InputNoteRecord {
    type Error = ClientError;

    fn try_into(self) -> Result<InputNote, Self::Error> {
        match self.inclusion_proof() {
            Some(proof) => Ok(InputNote::new(self.note().clone(), proof.clone())),
            None => Err(ClientError::NoteError(
                miden_objects::NoteError::invalid_origin_index(
                    "Input Note Record contains no inclusion proof".to_string(),
                ),
            )),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct NoteRecordDetails {
    nullifier: String,
    script: Vec<u8>,
    inputs: Vec<u8>,
    serial_num: Word,
}

impl NoteRecordDetails {
    fn new(nullifier: String, script: Vec<u8>, inputs: Vec<u8>, serial_num: Word) -> Self {
        Self {
            nullifier,
            script,
            inputs,
            serial_num,
        }
    }

    fn script(&self) -> &Vec<u8> {
        &self.script
    }

    fn inputs(&self) -> &Vec<u8> {
        &self.inputs
    }

    fn serial_num(&self) -> &Word {
        &self.serial_num
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
    /// Return a list of all [InputNoteRecord].
    All,
    /// Filter by consumed [InputNoteRecord]. notes that have been used as inputs in transactions.
    Consumed,
    /// Return a list of committed [InputNoteRecord]. These represent notes that the blockchain
    /// has included in a block, and for which we are storing anchor data.
    Committed,
    /// Return a list of pending [InputNoteRecord]. These represent notes for which the store
    /// does not have anchor data.
    Pending,
}

#[cfg(feature = "std")]
use std::{cell::RefCell, rc::Rc};

/// If std is enabled, implement the Store trait for `Rc<RefCell<T>>`
///
/// This allows a user to potentially create the Client with a single Store instance instead of two
/// separate ones.
///
/// # Example
///
/// ```ignore
/// use std::{cell::RefCell, rc::Rc};
/// use miden_client::client::Client;
/// use miden_client::client::rpc::NodeRpcClient;
/// use miden_client::store::Store;
///
/// pub fn shared_store_client<N: NodeRpcClient, S: Store>(
///     api: N,
///     store: S,
/// ) -> Result<Client<N, Rc<RefCell<S>>>, ClientError> {
///     let store = Rc::new(RefCell::new(store));
///
///     Client::new(api, store.clone(), store)
/// }
/// ```
#[cfg(feature = "std")]
impl<T: Store> Store for Rc<RefCell<T>> {
    fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        self.borrow().get_transactions(transaction_filter)
    }

    fn apply_transaction(&mut self, tx_result: TransactionResult) -> Result<(), StoreError> {
        self.borrow_mut().apply_transaction(tx_result)
    }

    // NOTE FUNCTIONS
    // ================================================================================================

    fn get_input_notes(&self, note_filter: NoteFilter) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.borrow().get_input_notes(note_filter)
    }

    fn get_output_notes(
        &self,
        note_filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.borrow().get_output_notes(note_filter)
    }

    fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError> {
        self.borrow().get_input_note(note_id)
    }

    fn insert_input_note(&mut self, note: &InputNoteRecord) -> Result<(), StoreError> {
        self.borrow_mut().insert_input_note(note)
    }

    // CHAIN DATA FUNCTIONS
    // ================================================================================================

    fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError> {
        self.borrow().get_block_headers(block_numbers)
    }

    fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        self.borrow().get_tracked_block_headers()
    }

    fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError> {
        self.borrow().get_chain_mmr_nodes(filter)
    }

    fn get_chain_mmr_peaks_by_block_num(&self, block_num: u32) -> Result<MmrPeaks, StoreError> {
        self.borrow().get_chain_mmr_peaks_by_block_num(block_num)
    }

    fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        self.borrow_mut()
            .insert_block_header(block_header, chain_mmr_peaks, has_client_notes)
    }

    // ACCOUNT FUNCTIONS
    // ================================================================================================

    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        self.borrow().get_account_ids()
    }

    fn get_account_stubs(&self) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError> {
        self.borrow().get_account_stubs()
    }

    fn get_account_stub(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError> {
        self.borrow().get_account_stub(account_id)
    }

    fn get_account(&self, account_id: AccountId) -> Result<(Account, Option<Word>), StoreError> {
        self.borrow().get_account(account_id)
    }

    fn get_account_auth(&self, account_id: AccountId) -> Result<AuthInfo, StoreError> {
        self.borrow().get_account_auth(account_id)
    }

    fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthInfo,
    ) -> Result<(), StoreError> {
        self.borrow_mut()
            .insert_account(account, account_seed, auth_info)
    }

    // SYNC-RELATED FUNCTIONS
    // ================================================================================================

    fn get_note_tags(&self) -> Result<Vec<u64>, StoreError> {
        self.borrow().get_note_tags()
    }

    fn add_note_tag(&mut self, tag: u64) -> Result<bool, StoreError> {
        self.borrow_mut().add_note_tag(tag)
    }

    fn get_sync_height(&self) -> Result<u32, StoreError> {
        self.borrow().get_sync_height()
    }

    fn apply_state_sync(
        &mut self,
        block_header: BlockHeader,
        nullifiers: Vec<Digest>,
        committed_notes: Vec<(NoteId, NoteInclusionProof)>,
        committed_transactions: &[TransactionId],
        new_mmr_peaks: MmrPeaks,
        new_authentication_nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError> {
        self.borrow_mut().apply_state_sync(
            block_header,
            nullifiers,
            committed_notes,
            committed_transactions,
            new_mmr_peaks,
            new_authentication_nodes,
        )
    }
}
