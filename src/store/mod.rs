use crate::{
    client::transactions::{TransactionResult, TransactionStub},
    errors::{ClientError, StoreError},
};
use assembly::ast::ModuleAst;
use clap::error::Result;
use crypto::{
    dsa::rpo_falcon512::KeyPair,
    merkle::{InOrderIndex, MmrPeaks},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Word,
};
use objects::{
    accounts::{Account, AccountId, AccountStorage, AccountStub},
    assets::Asset,
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
    fn get_note_tags(&self) -> Result<Vec<u64>, StoreError>;

    fn add_note_tag(&mut self, tag: u64) -> Result<bool, StoreError>;

    fn get_sync_height(&self) -> Result<u32, StoreError>;

    fn apply_state_sync(
        &mut self,
        block_header: BlockHeader,
        nullifiers: Vec<Digest>,
        committed_notes: Vec<(NoteId, NoteInclusionProof)>,
        new_mmr_peaks: MmrPeaks,
        new_authentication_nodes: &[(InOrderIndex, Digest)],
    ) -> Result<(), StoreError>;

    fn get_transactions(
        &self,
        transaction_filter: TransactionFilter,
    ) -> Result<Vec<TransactionStub>, StoreError>;

    fn insert_transaction_data(&mut self, tx_result: TransactionResult) -> Result<(), StoreError>;

    fn get_input_notes(
        &self,
        note_filter: InputNoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError>;

    fn get_input_note_by_id(&self, note_id: NoteId) -> Result<InputNoteRecord, StoreError>;

    fn insert_input_note(&mut self, note: &InputNoteRecord) -> Result<(), StoreError>;

    fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Digest>, StoreError>;

    fn insert_block_header(
        &self,
        block_header: BlockHeader,
        chain_mmr_peaks: MmrPeaks,
        has_client_notes: bool,
    ) -> Result<(), StoreError>;

    fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(BlockHeader, bool)>, StoreError>;

    fn get_block_header_by_num(&self, block_number: u32)
        -> Result<(BlockHeader, bool), StoreError>;

    fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError>;

    fn get_chain_mmr_nodes(
        &self,
        filter: ChainMmrNodeFilter,
    ) -> Result<BTreeMap<InOrderIndex, Digest>, StoreError>;

    fn get_chain_mmr_peaks_by_block_num(&self, block_num: u32) -> Result<MmrPeaks, StoreError>;

    // dudosa
    fn get_account_code(&self, root: Digest) -> Result<(Vec<Digest>, ModuleAst), StoreError>;

    // dudosa
    fn get_account_storage(&self, root: Digest) -> Result<AccountStorage, StoreError>;

    fn get_vault_assets(&self, root: Digest) -> Result<Vec<Asset>, StoreError>;

    fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Word,
        auth_info: &AuthInfo,
    ) -> Result<(), StoreError>;

    fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError>;

    fn get_accounts(&self) -> Result<Vec<(AccountStub, Option<Word>)>, StoreError>;

    fn get_account_stub_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), StoreError>;

    fn get_account_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), StoreError>;

    fn get_account_auth(&self, account_id: AccountId) -> Result<AuthInfo, StoreError>;
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
    All,
    List(&'a [InOrderIndex]),
}

// TRANSACTION FILTERS
// ================================================================================================

pub enum TransactionFilter {
    All,
    Uncomitted,
}

// NOTE FILTER
// ================================================================================================

pub enum InputNoteFilter {
    All,
    Consumed,
    Committed,
    Pending,
}
