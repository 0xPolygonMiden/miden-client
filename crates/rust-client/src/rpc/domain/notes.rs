use alloc::vec::Vec;

use miden_objects::{
    accounts::{Account, AccountId},
    crypto::merkle::MerklePath,
    notes::{Note, NoteExecutionHint, NoteId, NoteMetadata, NoteTag, NoteType},
    BlockHeader, Digest, Felt,
};

use super::MissingFieldHelper;
#[cfg(feature = "tonic")]
use crate::rpc::tonic_client::generated::note::NoteMetadata as ProtoNoteMetadata;
#[cfg(feature = "web-tonic")]
use crate::rpc::web_tonic_client::generated::note::NoteMetadata as ProtoNoteMetadata;
use crate::rpc::RpcConversionError;

impl TryFrom<ProtoNoteMetadata> for NoteMetadata {
    type Error = RpcConversionError;

    fn try_from(value: ProtoNoteMetadata) -> Result<Self, Self::Error> {
        let sender = value
            .sender
            .ok_or_else(|| ProtoNoteMetadata::missing_field("Sender"))?
            .try_into()?;
        let note_type = NoteType::try_from(value.note_type as u64)?;
        let tag = NoteTag::from(value.tag);
        let execution_hint_tag = (value.execution_hint & 0xff) as u8;
        let execution_hint_payload = ((value.execution_hint >> 8) & 0xffffff) as u32;
        let execution_hint =
            NoteExecutionHint::from_parts(execution_hint_tag, execution_hint_payload)?;

        let aux = Felt::try_from(value.aux).map_err(|_| RpcConversionError::NotAValidFelt)?;

        Ok(NoteMetadata::new(sender, note_type, tag, execution_hint, aux)?)
    }
}

impl From<NoteMetadata> for ProtoNoteMetadata {
    fn from(value: NoteMetadata) -> Self {
        ProtoNoteMetadata {
            sender: Some(value.sender().into()),
            note_type: value.note_type() as u32,
            tag: value.tag().into(),
            execution_hint: value.execution_hint().into(),
            aux: value.aux().into(),
        }
    }
}

// SYNC NOTE
// ================================================================================================

/// Represents a `SyncNoteResponse` with fields converted into domain types.
#[derive(Debug)]
pub struct NoteSyncInfo {
    /// Number of the latest block in the chain.
    pub chain_tip: u32,
    /// Block header of the block with the first note matching the specified criteria.
    pub block_header: BlockHeader,
    /// Proof for block header's MMR with respect to the chain tip.
    ///
    /// More specifically, the full proof consists of `forest`, `position` and `path` components.
    /// This value constitutes the `path`. The other two components can be obtained as follows:
    ///    - `position` is simply `resopnse.block_header.block_num`
    ///    - `forest` is the same as `response.chain_tip + 1`.
    pub mmr_path: MerklePath,
    /// List of all notes together with the Merkle paths from `response.block_header.note_root`.
    pub notes: Vec<CommittedNote>,
}

// COMMITTED NOTE
// ================================================================================================

/// Represents a committed note, returned as part of a `SyncStateResponse`.
#[derive(Debug, Clone)]
pub struct CommittedNote {
    /// Note ID of the committed note.
    note_id: NoteId,
    /// Note index for the note merkle tree.
    note_index: u16,
    /// Merkle path for the note merkle tree up to the block's note root.
    merkle_path: MerklePath,
    /// Note metadata.
    metadata: NoteMetadata,
}

impl CommittedNote {
    pub fn new(
        note_id: NoteId,
        note_index: u16,
        merkle_path: MerklePath,
        metadata: NoteMetadata,
    ) -> Self {
        Self {
            note_id,
            note_index,
            merkle_path,
            metadata,
        }
    }

    pub fn note_id(&self) -> &NoteId {
        &self.note_id
    }

    pub fn note_index(&self) -> u16 {
        self.note_index
    }

    pub fn merkle_path(&self) -> &MerklePath {
        &self.merkle_path
    }

    pub fn metadata(&self) -> NoteMetadata {
        self.metadata
    }
}

/// Contains information related to the note inclusion, but not related to the block header
/// that contains the note.
pub struct NoteInclusionDetails {
    /// Block number in which the note was included.
    pub block_num: u32,
    /// Index of the note in the block's note tree.
    pub note_index: u16,
    /// Merkle path to the note root of the block header.
    pub merkle_path: MerklePath,
}

impl NoteInclusionDetails {
    /// Creates a new [NoteInclusionDetails].
    pub fn new(block_num: u32, note_index: u16, merkle_path: MerklePath) -> Self {
        Self { block_num, note_index, merkle_path }
    }
}

/// Describes the possible responses from the `GetAccountDetails` endpoint for an account
pub enum AccountDetails {
    /// Private accounts are stored off-chain. Only a commitment to the state of the account is
    /// shared with the network. The full account state is to be tracked locally.
    Private(AccountId, AccountUpdateSummary),
    /// Public accounts are recorded on-chain. As such, its state is shared with the network and
    /// can always be retrieved through the appropriate RPC method.
    Public(Account, AccountUpdateSummary),
}

impl AccountDetails {
    /// Returns the account ID.
    pub fn account_id(&self) -> AccountId {
        match self {
            Self::Private(account_id, _) => *account_id,
            Self::Public(account, _) => account.id(),
        }
    }
}

/// Contains public updated information about the account requested.
pub struct AccountUpdateSummary {
    /// Hash of the account, that represents a commitment to its updated state.
    pub hash: Digest,
    /// Block number of last account update.
    pub last_block_num: u32,
}

impl AccountUpdateSummary {
    /// Creates a new [AccountUpdateSummary].
    pub fn new(hash: Digest, last_block_num: u32) -> Self {
        Self { hash, last_block_num }
    }
}

// NOTE DETAILS
// ================================================================================================

/// Describes the possible responses from  the `GetNotesById` endpoint for a single note.
#[allow(clippy::large_enum_variant)]
pub enum NoteDetails {
    /// Details for a private note only include its [NoteMetadata] and [NoteInclusionDetails].
    /// Other details needed to consume the note are expected to be stored locally, off-chain.
    Private(NoteId, NoteMetadata, NoteInclusionDetails),
    /// Contains the full [Note] object alongside its [NoteInclusionDetails].
    Public(Note, NoteInclusionDetails),
}

impl NoteDetails {
    /// Returns the note's inclusion details.
    pub fn inclusion_details(&self) -> &NoteInclusionDetails {
        match self {
            NoteDetails::Private(_, _, inclusion_details) => inclusion_details,
            NoteDetails::Public(_, inclusion_details) => inclusion_details,
        }
    }

    /// Returns the note's metadata.
    pub fn metadata(&self) -> &NoteMetadata {
        match self {
            NoteDetails::Private(_, metadata, _) => metadata,
            NoteDetails::Public(note, _) => note.metadata(),
        }
    }

    /// Returns the note's ID.
    pub fn id(&self) -> NoteId {
        match self {
            NoteDetails::Private(id, ..) => *id,
            NoteDetails::Public(note, _) => note.id(),
        }
    }
}
