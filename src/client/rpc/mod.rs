#![allow(async_fn_in_trait)]

use core::fmt;

use miden_objects::{
    accounts::{Account, AccountId},
    crypto::merkle::{MerklePath, MmrDelta, MmrProof},
    notes::{Note, NoteId, NoteMetadata, NoteTag},
    transaction::ProvenTransaction,
    BlockHeader, Digest,
};

use crate::errors::RpcError;

#[cfg(feature = "tonic")]
mod tonic_client;
#[cfg(feature = "tonic")]
pub use tonic_client::TonicRpcClient;

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
}

/// Describes the possible responses from the `GetAccountDetails` endpoint for an account
pub enum AccountDetails {
    OffChain(AccountId, AccountUpdateSummary),
    Public(Account, AccountUpdateSummary),
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
    /// For any NoteType::Offchain note, the return data is only the [NoteMetadata], whereas
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
    /// List of nullifiers that identify spent notes
    pub nullifiers: Vec<Digest>,
}

// COMMITTED NOTE
// ================================================================================================

/// Represents a committed note, returned as part of a `SyncStateResponse`
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
        }
    }
}
