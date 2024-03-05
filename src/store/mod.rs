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
    notes::{
        Note, NoteAssets, NoteId, NoteInclusionProof, NoteInputs, NoteMetadata, NoteScript,
        Nullifier,
    },
    transaction::{InputNote, TransactionId},
    utils::collections::BTreeMap,
    BlockHeader, Digest, NoteError,
};
use serde::{Deserialize, Serialize};

pub mod data_store;
pub mod sqlite_store;

#[cfg(any(test, feature = "mock"))]
pub mod mock_executor_data_store;

// STORE TRAIT
// ================================================================================================

pub trait Store {
    // TRANSACTIONS
    // --------------------------------------------------------------------------------------------

    /// Retrieves stored transactions, filtered by [TransactionFilter].
    fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError>;

    /// Inserts a transaction and updates the current state based on the `tx_result` changes
    fn apply_transaction(&mut self, tx_result: TransactionResult) -> Result<(), StoreError>;

    // NOTES
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes from the store
    fn get_input_notes(&self, filter: NoteFilter) -> Result<Vec<InputNoteRecord>, StoreError>;

    /// Retrieves the output notes from the store
    fn get_output_notes(&self, filter: NoteFilter) -> Result<Vec<OutputNoteRecord>, StoreError>;

    /// Retrieves an [InputNoteRecord] for the input note corresponding to the specified id from
    /// the store.
    ///
    /// # Errors
    ///
    /// Returns a [StoreError::InputNoteNotFound] if there is no Note with the provided id
    fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError>;

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

    /// Update previously-existing account after a transaction execution
    /// The account that is to be updated is identified by the Account ID
    fn update_account(&mut self, new_account_state: Account) -> Result<(), StoreError>;

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

// NOTE STATUS
// ================================================================================================
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NoteStatus {
    Pending,
    Committed,
    Consumed,
}

impl From<NoteStatus> for u8 {
    fn from(value: NoteStatus) -> Self {
        match value {
            NoteStatus::Pending => 0,
            NoteStatus::Committed => 1,
            NoteStatus::Consumed => 2,
        }
    }
}

impl From<u8> for NoteStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => NoteStatus::Pending,
            1 => NoteStatus::Committed,
            _ => NoteStatus::Consumed,
        }
    }
}

impl Serializable for NoteStatus {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_bytes(&[(*self).into()]);
    }
}

impl Deserializable for NoteStatus {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let enum_byte = u8::read_from(source)?;
        Ok(enum_byte.into())
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
    id: NoteId,
    recipient: Digest,
    assets: NoteAssets,
    status: NoteStatus,
    metadata: Option<NoteMetadata>,
    inclusion_proof: Option<NoteInclusionProof>,
    details: NoteRecordDetails,
}

impl InputNoteRecord {
    pub fn new(
        id: NoteId,
        recipient: Digest,
        assets: NoteAssets,
        status: NoteStatus,
        metadata: Option<NoteMetadata>,
        inclusion_proof: Option<NoteInclusionProof>,
        details: NoteRecordDetails,
    ) -> InputNoteRecord {
        InputNoteRecord {
            id,
            recipient,
            assets,
            status,
            metadata,
            inclusion_proof,
            details,
        }
    }

    pub fn id(&self) -> NoteId {
        self.id
    }

    pub fn recipient(&self) -> Digest {
        self.recipient
    }

    pub fn assets(&self) -> &NoteAssets {
        &self.assets
    }

    pub fn status(&self) -> NoteStatus {
        self.status
    }

    pub fn metadata(&self) -> Option<&NoteMetadata> {
        self.metadata.as_ref()
    }

    pub fn nullifier(&self) -> &str {
        &self.details.nullifier
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        self.inclusion_proof.as_ref()
    }

    pub fn details(&self) -> &NoteRecordDetails {
        &self.details
    }

    pub fn note(&self) -> Option<&Note> {
        // TODO: add logic to return Some(note) if we have enough info to build one
        None
    }
}

impl Serializable for InputNoteRecord {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.id().write_into(target);
        self.recipient().write_into(target);
        self.assets().write_into(target);
        self.status().write_into(target);
        self.metadata().write_into(target);
        self.details().write_into(target);
        self.inclusion_proof().write_into(target);
    }
}

impl Deserializable for InputNoteRecord {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let id = NoteId::read_from(source)?;
        let recipient = Digest::read_from(source)?;
        let assets = NoteAssets::read_from(source)?;
        let status = NoteStatus::read_from(source)?;
        let metadata = Option::<NoteMetadata>::read_from(source)?;
        let details = NoteRecordDetails::read_from(source)?;
        let inclusion_proof = Option::<NoteInclusionProof>::read_from(source)?;

        Ok(InputNoteRecord {
            id,
            recipient,
            assets,
            status,
            metadata,
            inclusion_proof,
            details,
        })
    }
}

impl From<Note> for InputNoteRecord {
    fn from(note: Note) -> Self {
        InputNoteRecord {
            id: note.id(),
            recipient: note.recipient(),
            assets: note.assets().clone(),
            status: NoteStatus::Pending,
            metadata: Some(*note.metadata()),
            inclusion_proof: None,
            details: NoteRecordDetails {
                nullifier: note.nullifier().to_string(),
                script: note.script().to_bytes(),
                inputs: note.inputs().to_bytes(),
                serial_num: note.serial_num(),
            },
        }
    }
}

impl From<InputNote> for InputNoteRecord {
    fn from(recorded_note: InputNote) -> Self {
        InputNoteRecord {
            id: recorded_note.note().id(),
            recipient: recorded_note.note().recipient(),
            assets: recorded_note.note().assets().clone(),
            status: NoteStatus::Pending,
            metadata: Some(*recorded_note.note().metadata()),
            details: NoteRecordDetails {
                nullifier: recorded_note.note().nullifier().to_string(),
                script: recorded_note.note().script().to_bytes(),
                inputs: recorded_note.note().inputs().to_bytes(),
                serial_num: recorded_note.note().serial_num(),
            },
            inclusion_proof: Some(recorded_note.proof().clone()),
        }
    }
}

impl TryInto<InputNote> for InputNoteRecord {
    type Error = ClientError;

    fn try_into(self) -> Result<InputNote, Self::Error> {
        match (self.inclusion_proof, self.metadata) {
            (Some(proof), Some(metadata)) => {
                let script = NoteScript::read_from_bytes(&self.details.script).map_err(|err| {
                    ClientError::NoteError(NoteError::NoteDeserializationError(err))
                })?;
                let inputs = NoteInputs::read_from_bytes(&self.details.inputs).map_err(|err| {
                    ClientError::NoteError(NoteError::NoteDeserializationError(err))
                })?;
                let note = Note::from_parts(
                    script,
                    inputs,
                    self.assets,
                    self.details.serial_num,
                    metadata,
                );
                Ok(InputNote::new(note, proof.clone()))
            }
            (None, _) => Err(ClientError::NoteError(
                objects::NoteError::invalid_origin_index(
                    "Input Note Record contains no proof".to_string(),
                ),
            )),
            (_, None) => Err(ClientError::NoteError(
                // TODO: use better error?
                objects::NoteError::invalid_origin_index(
                    "Input Note Record contains no metadata".to_string(),
                ),
            )),
        }
    }
}

// OUTPUT NOTE RECORD
// ================================================================================================

/// Represents a Note which was the result of executing some transaction of which the [Store] can
/// keep track and retrieve.
///
/// An [OutputNoteRecord] contains all the information of a [Note] while it allows for not
/// knowing the details (nullifier, script, inputs and serial number), in addition of (optionally)
/// the [NoteInclusionProof] that identifies when the note was included in the chain.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputNoteRecord {
    id: NoteId,
    recipient: Digest,
    assets: NoteAssets,
    status: NoteStatus,
    metadata: NoteMetadata,
    inclusion_proof: Option<NoteInclusionProof>,
    details: Option<NoteRecordDetails>,
}

impl OutputNoteRecord {
    pub fn new(
        id: NoteId,
        recipient: Digest,
        assets: NoteAssets,
        status: NoteStatus,
        metadata: NoteMetadata,
        inclusion_proof: Option<NoteInclusionProof>,
        details: Option<NoteRecordDetails>,
    ) -> OutputNoteRecord {
        OutputNoteRecord {
            id,
            recipient,
            assets,
            status,
            metadata,
            inclusion_proof,
            details,
        }
    }

    pub fn id(&self) -> NoteId {
        self.id
    }

    pub fn recipient(&self) -> Digest {
        self.recipient
    }

    pub fn assets(&self) -> &NoteAssets {
        &self.assets
    }

    pub fn status(&self) -> NoteStatus {
        self.status
    }

    pub fn metadata(&self) -> &NoteMetadata {
        &self.metadata
    }

    pub fn inclusion_proof(&self) -> Option<&NoteInclusionProof> {
        self.inclusion_proof.as_ref()
    }

    pub fn details(&self) -> Option<&NoteRecordDetails> {
        self.details.as_ref()
    }

    pub fn note(&self) -> Option<&Note> {
        // TODO: add logic to return Some(note) if we have enough info to build one
        None
    }
}

impl From<Note> for OutputNoteRecord {
    fn from(note: Note) -> Self {
        OutputNoteRecord {
            id: note.id(),
            recipient: note.recipient(),
            assets: note.assets().clone(),
            status: NoteStatus::Pending,
            metadata: *note.metadata(),
            inclusion_proof: None,
            details: Some(NoteRecordDetails {
                nullifier: note.nullifier().to_string(),
                script: note.script().to_bytes(),
                inputs: note.inputs().to_bytes(),
                serial_num: note.serial_num(),
            }),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NoteRecordDetails {
    nullifier: String,
    script: Vec<u8>,
    inputs: Vec<u8>,
    serial_num: Word,
}

impl NoteRecordDetails {
    pub fn new(nullifier: String, script: Vec<u8>, inputs: Vec<u8>, serial_num: Word) -> Self {
        Self {
            nullifier,
            script,
            inputs,
            serial_num,
        }
    }

    pub fn script(&self) -> &Vec<u8> {
        &self.script
    }

    pub fn inputs(&self) -> &Vec<u8> {
        &self.inputs
    }

    pub fn serial_num(&self) -> Word {
        self.serial_num
    }
}

impl Serializable for NoteRecordDetails {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let nullifier_bytes = self.nullifier.as_bytes();
        target.write_usize(nullifier_bytes.len());
        target.write_bytes(nullifier_bytes);

        target.write_usize(self.script().len());
        target.write_bytes(self.script());

        target.write_usize(self.inputs().len());
        target.write_bytes(self.inputs());

        self.serial_num().write_into(target);
    }
}

impl Deserializable for NoteRecordDetails {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let nullifier_len = usize::read_from(source)?;
        let nullifier_bytes = source.read_vec(nullifier_len)?;
        let nullifier =
            String::from_utf8(nullifier_bytes).expect("Nullifier String bytes should be readable.");

        let script_len = usize::read_from(source)?;
        let script = source.read_vec(script_len)?;

        let inputs_len = usize::read_from(source)?;
        let inputs = source.read_vec(inputs_len)?;

        let serial_num = Word::read_from(source)?;

        Ok(NoteRecordDetails::new(
            nullifier, script, inputs, serial_num,
        ))
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
}
