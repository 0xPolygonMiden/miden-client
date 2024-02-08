use crate::{
    client::transactions::{TransactionRecord, TransactionResult},
    errors::{ClientError, StoreError},
};
use clap::error::Result;
use crypto::{
    dsa::rpo_falcon512::KeyPair,
    merkle::{InOrderIndex, MmrPeaks},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Word,
};
use objects::{
    accounts::{Account, AccountId, AccountStub},
    notes::{Note, NoteId, NoteInclusionProof},
    transaction::InputNote,
    utils::collections::BTreeMap,
    BlockHeader, Digest,
};

pub mod data_store;
pub mod sqlite_store;

#[cfg(any(test, feature = "mock"))]
pub mod mock_executor_data_store;

// STORE TRAIT
// ================================================================================================

pub trait Store {
    // TRANSACTIONS
    // ================================================================================================

    /// Retrieves all executed transactions from the database
    fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError>;

    /// Inserts a transaction and updates the current state based on the `tx_result` changes
    fn insert_transaction_data(&mut self, tx_result: TransactionResult) -> Result<(), StoreError>;

    // NOTE FUNCTIONS
    // ================================================================================================

    /// Retrieves the input notes from the database
    fn get_input_notes(
        &self,
        note_filter: InputNoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError>;

    /// Retrieves the input note with the specified id from the database
    fn get_input_note_by_id(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError>;

    /// Returns the nullifiers of all unspent input notes
    fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Digest>, StoreError>;

    /// Inserts the provided input note into the database
    fn insert_input_note(&mut self, note: &InputNoteRecord) -> Result<(), StoreError>;

    // CHAIN DATA FUNCTIONS
    // ================================================================================================

    /// Retrieves a list of [BlockHeader] by number and a boolean value that represents whether the
    /// block contains notes relevant to the client. It's up to the callee to check that all
    /// requested block headers were found
    fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError>;

    /// Retrieves a [BlockHeader] by number and a boolean value that represents whether the
    /// block contains notes relevant to the client.
    fn get_block_header_by_num(&self, block_number: u32)
        -> Result<(BlockHeader, bool), StoreError>;

    /// Retrieves a list of [BlockHeader] that include relevant notes to the client.
    fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError>;

    /// Retrieves all MMR authentication nodes based on [ChainMmrNodeFilter].
    fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError>;

    /// Returns peaks information from the blockchain by a specific block number.
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

    // ACCOUNT FUNCTIONS
    // ================================================================================================

    /// Returns the account IDs of all accounts stored in the database
    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError>;

    /// Returns a list of [AccountStub] of all accounts stored in the database along with the seeds
    /// used to create them.
    ///
    /// Said accounts' state is the state after the last performed sync.
    fn get_accounts(&self) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError>;

    /// Retrieves an [AccountStub] object for the specified [AccountId] along with the seed
    /// used to create it.
    ///
    /// Said account's state is the state according to the last sync performed.
    ///
    /// # Errors
    /// Returns an [Err] if the account was not found
    fn get_account_stub_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError>;

    /// Retrieves a full [Account] object, along with its seed.
    ///
    /// This function returns the [Account]'s latest state. If the account is new (that is, has
    /// never executed a trasaction), the returned seed will be `Some(Word)`; otherwise the seed
    /// will be `None`
    fn get_account_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), StoreError>;

    /// Inserts an [Account] along with the seed used to create it and its [AuthInfo]
    fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Word,
        auth_info: &AuthInfo,
    ) -> Result<(), StoreError>;

    // SYNC-RELATED FUNCTIONS
    // ================================================================================================

    /// Returns the note tags that the client is interested in.
    fn get_note_tags(&self) -> Result<Vec<u64>, StoreError>; // TODO: Should this go away?

    /// Adds a note tag to the list of tags that the client is interested in.
    fn add_note_tag(&mut self, tag: u64) -> Result<bool, StoreError>; // TODO: Should this go away?

    /// Returns the block number of the last state sync block.
    fn get_sync_height(&self) -> Result<u32, StoreError>;

    /// Applies the state sync update to the store. An update involves:
    ///
    /// - Inserting the new block header to the store alongside new MMR peaks information
    /// - Updating the notes, marking them as `committed` or `consumed` based on incoming
    ///   inclusion proofs and nullifiers
    /// - Storing new MMR authentication nodes
    fn apply_state_sync(
        &mut self,
        block_header: BlockHeader,
        nullifiers: Vec<Digest>,
        committed_notes: Vec<(NoteId, NoteInclusionProof)>,
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
    fn write_into<W: crypto::utils::ByteWriter>(&self, target: &mut W) {
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
    fn read_from<R: crypto::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, crypto::utils::DeserializationError> {
        let auth_type: u8 = source.read_u8()?;
        match auth_type {
            RPO_FALCON512_AUTH => {
                let key_pair = KeyPair::read_from(source)?;
                Ok(AuthInfo::RpoFalcon512(key_pair))
            }
            val => Err(crypto::utils::DeserializationError::InvalidValue(
                val.to_string(),
            )),
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
        target.write(self.note().to_bytes());
        target.write(self.inclusion_proof.to_bytes());
    }
}

impl Deserializable for InputNoteRecord {
    fn read_from<R: ByteReader>(
        source: &mut R,
    ) -> std::prelude::v1::Result<Self, DeserializationError> {
        let note: Note = source.read()?;
        let proof: Option<NoteInclusionProof> = source.read()?;
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
                objects::NoteError::invalid_origin_index(
                    "Input Note Record contains no inclusion proof".to_string(),
                ),
            )),
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

pub enum InputNoteFilter {
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
