#![allow(async_fn_in_trait)]

use alloc::vec::Vec;
use core::fmt;

mod errors;
pub(crate) use errors::RpcConversionError;
pub use errors::RpcError;
use miden_objects::{
    accounts::{Account, AccountId},
    crypto::merkle::{MerklePath, MmrDelta, MmrProof},
    notes::{Note, NoteId, NoteMetadata, NoteTag, Nullifier},
    transaction::{ProvenTransaction, TransactionId},
    BlockHeader, Digest,
};

#[cfg(all(feature = "tonic", feature = "web-tonic"))]
compile_error!("features `tonic` and `web-tonic` are mutually exclusive");

#[cfg(any(feature = "tonic", feature = "web-tonic"))]
mod domain;

#[cfg(feature = "tonic")]
mod tonic_client;
#[cfg(test)]
pub use tonic_client::generated;
#[cfg(feature = "tonic")]
pub use tonic_client::TonicRpcClient;

#[cfg(feature = "web-tonic")]
mod web_tonic_client;
#[cfg(feature = "web-tonic")]
pub use web_tonic_client::WebTonicRpcClient;

// NOTE DETAILS
// ================================================================================================

/// Describes the possible responses from  the `GetNotesById` endpoint for a single note
pub enum NoteDetails {
    OffChain(NoteId, NoteMetadata, NoteInclusionDetails),
    Public(Note, NoteInclusionDetails),
}

impl NoteDetails {
    pub fn inclusion_details(&self) -> &NoteInclusionDetails {
        match self {
            NoteDetails::OffChain(_, _, inclusion_details) => inclusion_details,
            NoteDetails::Public(_, inclusion_details) => inclusion_details,
        }
    }

    pub fn metadata(&self) -> &NoteMetadata {
        match self {
            NoteDetails::OffChain(_, metadata, _) => metadata,
            NoteDetails::Public(note, _) => note.metadata(),
        }
    }

    pub fn id(&self) -> NoteId {
        match self {
            NoteDetails::OffChain(id, ..) => *id,
            NoteDetails::Public(note, _) => note.id(),
        }
    }
}

/// Describes the possible responses from the `GetAccountDetails` endpoint for an account
pub enum AccountDetails {
    OffChain(AccountId, AccountUpdateSummary),
    Public(Account, AccountUpdateSummary),
}

impl AccountDetails {
    pub fn account_id(&self) -> AccountId {
        match self {
            Self::OffChain(account_id, _) => *account_id,
            Self::Public(account, _) => account.id(),
        }
    }
}

/// Contains public updated information about the account requested
pub struct AccountUpdateSummary {
    /// Account hash
    pub hash: Digest,
    /// Block number of last account update
    pub last_block_num: u32,
}

impl AccountUpdateSummary {
    pub fn new(hash: Digest, last_block_num: u32) -> Self {
        Self { hash, last_block_num }
    }
}

/// Contains information related to the note inclusion, but not related to the block header
/// that contains the note
pub struct NoteInclusionDetails {
    pub block_num: u32,
    pub note_index: u32,
    pub merkle_path: MerklePath,
}

impl NoteInclusionDetails {
    pub fn new(block_num: u32, note_index: u32, merkle_path: MerklePath) -> Self {
        Self { block_num, note_index, merkle_path }
    }
}

// NODE RPC CLIENT TRAIT
// ================================================================================================

/// Defines the interface for communicating with the Miden node.
///
/// The implementers are responsible for connecting to the Miden node, handling endpoint
/// requests/responses, and translating responses into domain objects relevant for each of the
/// endpoints.
pub trait NodeRpcClient {
    /// Given a Proven Transaction, send it to the node for it to be included in a future block
    /// using the `/SubmitProvenTransaction` rpc endpoint
    async fn submit_proven_transaction(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), RpcError>;

    /// Given a block number, fetches the block header corresponding to that height from the node
    /// using the `/GetBlockHeaderByNumber` endpoint.
    /// If `include_mmr_proof` is set to true and the function returns an `Ok`, the second value
    /// of the return tuple should always be Some(MmrProof)   
    ///
    /// When `None` is provided, returns info regarding the latest block
    async fn get_block_header_by_number(
        &mut self,
        block_num: Option<u32>,
        include_mmr_proof: bool,
    ) -> Result<(BlockHeader, Option<MmrProof>), RpcError>;

    /// Fetches note-related data for a list of [NoteId] using the `/GetNotesById` rpc endpoint
    ///
    /// For any NoteType::Private note, the return data is only the [NoteMetadata], whereas
    /// for NoteType::Onchain notes, the return data includes all details.
    async fn get_notes_by_id(&mut self, note_ids: &[NoteId]) -> Result<Vec<NoteDetails>, RpcError>;

    /// Fetches info from the node necessary to perform a state sync using the
    /// `/SyncState` rpc endpoint
    ///
    /// - `block_num` is the last block number known by the client. The returned [StateSyncInfo]
    ///   should contain data starting from the next block, until the first block which contains a
    ///   note of matching the requested tag, or the chain tip if there are no notes.
    /// - `account_ids` is a list of account ids and determines the accounts the client is interested
    ///   in and should receive account updates of.
    /// - `note_tags` is a list of tags used to filter the notes the client is interested in, which
    ///   serves as a "note group" filter. Notice that you can't filter by a specific note id
    /// - `nullifiers_tags` similar to `note_tags`, is a list of tags used to filter the nullifiers
    ///   corresponding to some notes the client is interested in
    async fn sync_state(
        &mut self,
        block_num: u32,
        account_ids: &[AccountId],
        note_tags: &[NoteTag],
        nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, RpcError>;

    /// Fetches the current state of an account from the node using the `/GetAccountDetails` rpc endpoint
    ///
    /// - `account_id` is the id of the wanted account.
    async fn get_account_update(
        &mut self,
        account_id: AccountId,
    ) -> Result<AccountDetails, RpcError>;

    async fn sync_notes(
        &mut self,
        block_num: u32,
        note_tags: &[NoteTag],
    ) -> Result<NoteSyncInfo, RpcError>;
}

// SYNC NOTE
// ================================================================================================

/// Represents a `SyncNoteResponse` with fields converted into domain types
#[derive(Debug)]
pub struct NoteSyncInfo {
    /// Number of the latest block in the chain
    pub chain_tip: u32,
    /// Block header of the block with the first note matching the specified criteria
    pub block_header: Option<BlockHeader>,
    /// Proof for block header's MMR with respect to the chain tip.
    ///
    /// More specifically, the full proof consists of `forest`, `position` and `path` components. This
    /// value constitutes the `path`. The other two components can be obtained as follows:
    ///    - `position` is simply `resopnse.block_header.block_num`
    ///    - `forest` is the same as `response.chain_tip + 1`
    pub mmr_path: Option<MerklePath>,
    /// List of all notes together with the Merkle paths from `response.block_header.note_root`
    pub notes: Vec<CommittedNote>,
}

// STATE SYNC INFO
// ================================================================================================

/// Represents a `SyncStateResponse` with fields converted into domain types
pub struct StateSyncInfo {
    /// The block number of the chain tip at the moment of the response
    pub chain_tip: u32,
    /// The returned block header
    pub block_header: BlockHeader,
    /// MMR delta that contains data for (current_block.num, incoming_block_header.num-1)
    pub mmr_delta: MmrDelta,
    /// Tuples of AccountId alongside their new account hashes
    pub account_hash_updates: Vec<(AccountId, Digest)>,
    /// List of tuples of Note ID, Note Index and Merkle Path for all new notes
    pub note_inclusions: Vec<CommittedNote>,
    /// List of nullifiers that identify spent notes along with the block number at which they were
    /// consumed
    pub nullifiers: Vec<NullifierUpdate>,
    /// List of transaction IDs of transaction that were included in (request.block_num,
    /// response.block_num-1) along with the account the tx was executed against and the block
    /// number the transaction was included in.
    pub transactions: Vec<TransactionUpdate>,
}

/// Represents a transaction that was included in the node at a certain block.
pub struct TransactionUpdate {
    /// The transaction Identifier
    pub transaction_id: TransactionId,
    /// The number of the block in which the transaction was included
    pub block_num: u32,
    /// The account that the transcation was executed against
    pub account_id: AccountId,
}

/// Represents a note that was consumed in the node at a certain block.
pub struct NullifierUpdate {
    /// The nullifier of the consumed note
    pub nullifier: Nullifier,
    /// The number of the block in which the note consumption was registered.
    pub block_num: u32,
}

// COMMITTED NOTE
// ================================================================================================

/// Represents a committed note, returned as part of a `SyncStateResponse`
#[derive(Debug, Clone)]
pub struct CommittedNote {
    /// Note ID of the committed note
    note_id: NoteId,
    /// Note index for the note merkle tree
    note_index: u32,
    /// Merkle path for the note merkle tree up to the block's note root
    merkle_path: MerklePath,
    /// Note metadata
    metadata: NoteMetadata,
}

impl CommittedNote {
    pub fn new(
        note_id: NoteId,
        note_index: u32,
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

    pub fn note_index(&self) -> u32 {
        self.note_index
    }

    pub fn merkle_path(&self) -> &MerklePath {
        &self.merkle_path
    }

    pub fn metadata(&self) -> NoteMetadata {
        self.metadata
    }
}

// RPC API ENDPOINT
// ================================================================================================
//
#[derive(Debug)]
pub enum NodeRpcClientEndpoint {
    GetAccountDetails,
    GetBlockHeaderByNumber,
    SyncState,
    SubmitProvenTx,
    SyncNotes,
}

impl fmt::Display for NodeRpcClientEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeRpcClientEndpoint::GetAccountDetails => write!(f, "get_account_details"),
            NodeRpcClientEndpoint::GetBlockHeaderByNumber => {
                write!(f, "get_block_header_by_number")
            },
            NodeRpcClientEndpoint::SyncState => write!(f, "sync_state"),
            NodeRpcClientEndpoint::SubmitProvenTx => write!(f, "submit_proven_transaction"),
            NodeRpcClientEndpoint::SyncNotes => write!(f, "sync_notes"),
        }
    }
}
