//! Provides an interface for the client to communicate with Miden nodes using
//! Remote Procedure Calls (RPC). It facilitates syncing with the network and submitting
//! transactions.

use alloc::{boxed::Box, vec::Vec};
use core::fmt;

use async_trait::async_trait;

mod errors;
pub(crate) use errors::RpcConversionError;
pub use errors::RpcError;
use miden_objects::{
    accounts::{Account, AccountHeader, AccountId, AccountStorageHeader},
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

use crate::sync::get_nullifier_prefix;

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

// ACCOUNT PROOF
// ================================================================================================

/// Represents a proof of existence of an account's state at a specific block number.
pub struct AccountProof {
    /// Account ID.
    account_id: AccountId,
    /// Block number at which the state is being represented.
    block_num: u32,
    /// Authentication path from the `account_root` of the block header to the account.
    merkle_proof: MerklePath,
    /// Account hash for the current state.
    account_hash: Digest,
    /// State headers of public accounts.
    state_headers: Option<(AccountHeader, AccountStorageHeader)>,
}

impl AccountProof {
    pub fn new(
        account_id: AccountId,
        block_num: u32,
        merkle_proof: MerklePath,
        account_hash: Digest,
        state_headers: Option<(AccountHeader, AccountStorageHeader)>,
    ) -> Self {
        Self {
            account_id,
            block_num,
            merkle_proof,
            account_hash,
            state_headers,
        }
    }

    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    pub fn state_headers(&self) -> Option<&(AccountHeader, AccountStorageHeader)> {
        self.state_headers.as_ref()
    }

    pub fn account_header(&self) -> Option<&AccountHeader> {
        self.state_headers.as_ref().map(|headers| &headers.0)
    }

    pub fn storage_header(&self) -> Option<&AccountStorageHeader> {
        self.state_headers.as_ref().map(|headers| &headers.1)
    }

    pub fn account_hash(&self) -> Digest {
        self.account_hash
    }

    pub fn block_num(&self) -> u32 {
        self.block_num
    }

    pub fn merkle_proof(&self) -> &MerklePath {
        &self.merkle_proof
    }
}

// NODE RPC CLIENT TRAIT
// ================================================================================================

/// Defines the interface for communicating with the Miden node.
///
/// The implementers are responsible for connecting to the Miden node, handling endpoint
/// requests/responses, and translating responses into domain objects relevant for each of the
/// endpoints.
#[async_trait(?Send)]
pub trait NodeRpcClient {
    /// Given a Proven Transaction, send it to the node for it to be included in a future block
    /// using the `/SubmitProvenTransaction` RPC endpoint.
    async fn submit_proven_transaction(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), RpcError>;

    /// Given a block number, fetches the block header corresponding to that height from the node
    /// using the `/GetBlockHeaderByNumber` endpoint.
    /// If `include_mmr_proof` is set to true and the function returns an `Ok`, the second value
    /// of the return tuple should always be Some(MmrProof)   
    ///
    /// When `None` is provided, returns info regarding the latest block.
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
    /// `/SyncState` RPC endpoint
    ///
    /// - `block_num` is the last block number known by the client. The returned [StateSyncInfo]
    ///   should contain data starting from the next block, until the first block which contains a
    ///   note of matching the requested tag, or the chain tip if there are no notes.
    /// - `account_ids` is a list of account ids and determines the accounts the client is
    ///   interested in and should receive account updates of.
    /// - `note_tags` is a list of tags used to filter the notes the client is interested in, which
    ///   serves as a "note group" filter. Notice that you can't filter by a specific note id
    /// - `nullifiers_tags` similar to `note_tags`, is a list of tags used to filter the nullifiers
    ///   corresponding to some notes the client is interested in.
    async fn sync_state(
        &mut self,
        block_num: u32,
        account_ids: &[AccountId],
        note_tags: &[NoteTag],
        nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, RpcError>;

    /// Fetches the current state of an account from the node using the `/GetAccountDetails` RPC
    /// endpoint
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

    /// Fetches the nullifiers corresponding to a list of prefixes using the
    /// `/CheckNullifiersByPrefix` RPC endpoint.
    async fn check_nullifiers_by_prefix(
        &mut self,
        prefix: &[u16],
    ) -> Result<Vec<(Nullifier, u32)>, RpcError>;

    /// Fetches the current account state, using th `/GetAccountProofs` RPC endpoint.
    async fn get_account_proofs(
        &mut self,
        account_ids: &[AccountId],
        includ_headers: bool,
    ) -> Result<Vec<AccountProof>, RpcError>;

    /// Fetches the commit height where the nullifier was consumed. If the nullifier is not found,
    /// then `None` is returned.
    ///
    /// The default implementation of this method uses [NodeRpcClient::check_nullifiers_by_prefix].
    async fn get_nullifier_commit_height(
        &mut self,
        nullifier: &Nullifier,
    ) -> Result<Option<u32>, RpcError> {
        let nullifiers =
            self.check_nullifiers_by_prefix(&[get_nullifier_prefix(nullifier)]).await?;

        Ok(nullifiers.iter().find(|(n, _)| n == nullifier).map(|(_, block_num)| *block_num))
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

// STATE SYNC INFO
// ================================================================================================

/// Represents a `SyncStateResponse` with fields converted into domain types.
pub struct StateSyncInfo {
    /// The block number of the chain tip at the moment of the response.
    pub chain_tip: u32,
    /// The returned block header.
    pub block_header: BlockHeader,
    /// MMR delta that contains data for (current_block.num, incoming_block_header.num-1).
    pub mmr_delta: MmrDelta,
    /// Tuples of AccountId alongside their new account hashes.
    pub account_hash_updates: Vec<(AccountId, Digest)>,
    /// List of tuples of Note ID, Note Index and Merkle Path for all new notes.
    pub note_inclusions: Vec<CommittedNote>,
    /// List of nullifiers that identify spent notes along with the block number at which they were
    /// consumed.
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
    /// The number of the block in which the transaction was included.
    pub block_num: u32,
    /// The account that the transcation was executed against.
    pub account_id: AccountId,
}

/// Represents a note that was consumed in the node at a certain block.
pub struct NullifierUpdate {
    /// The nullifier of the consumed note.
    pub nullifier: Nullifier,
    /// The number of the block in which the note consumption was registered.
    pub block_num: u32,
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

// RPC API ENDPOINT
// ================================================================================================
//
/// RPC methods for the Miden protocol.
#[derive(Debug)]
pub enum NodeRpcClientEndpoint {
    CheckNullifiersByPrefix,
    GetAccountDetails,
    GetAccountProofs,
    GetBlockHeaderByNumber,
    SyncState,
    SubmitProvenTx,
    SyncNotes,
}

impl fmt::Display for NodeRpcClientEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeRpcClientEndpoint::CheckNullifiersByPrefix => {
                write!(f, "check_nullifiers_by_prefix")
            },
            NodeRpcClientEndpoint::GetAccountDetails => write!(f, "get_account_details"),
            NodeRpcClientEndpoint::GetAccountProofs => write!(f, "get_account_proofs"),
            NodeRpcClientEndpoint::GetBlockHeaderByNumber => {
                write!(f, "get_block_header_by_number")
            },
            NodeRpcClientEndpoint::SyncState => write!(f, "sync_state"),
            NodeRpcClientEndpoint::SubmitProvenTx => write!(f, "submit_proven_transaction"),
            NodeRpcClientEndpoint::SyncNotes => write!(f, "sync_notes"),
        }
    }
}
