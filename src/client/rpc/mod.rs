use crate::errors::NodeRpcClientError;
use async_trait::async_trait;
use core::fmt;
use crypto::merkle::{MerklePath, MmrDelta};
use objects::{
    accounts::AccountId,
    notes::{NoteId, NoteMetadata},
    transaction::ProvenTransaction,
    BlockHeader, Digest,
};

mod tonic_client;
pub use tonic_client::TonicRpcClient;

// NODE API TRAIT
// ================================================================================================

#[async_trait]
/// This trait is mean to be used to abstract the connection between the client and the node. Since
/// different environments (e.g a mocked client, compiling to wasm or no_std, using other crate for
/// the communication) may require other implementations it made sense to create a trait for this
pub trait NodeRpcClient {
    /// Given a Proven Transaction, send it to the node for it to be included in a future block
    async fn submit_proven_transaction(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), NodeRpcClientError>;

    /// Given a block number, fetch the block header corresponding to that height from the node
    ///
    /// When `None` is provided, returns info regarding the latest block
    async fn get_block_header_by_number(
        &mut self,
        block_number: Option<u32>,
    ) -> Result<BlockHeader, NodeRpcClientError>;

    /// Fetch info from the node necessary to perform a state sync
    ///
    /// - `block_num` is the last block number known by the client. The returned [ StateSyncInfo ]
    /// should contain data starting from the next block, until the first block which contains a
    /// note of matching the requested tag, or the chain tip if there are no notes.
    /// - `account_ids` is a list of account ids and determines the accounts the client is interested
    /// in and should receive account updates of.
    /// - `note_tags` is a list of tags used to filter the notes the client is interested in, which
    /// serves as a "note group" filter. Notice that you can't filter by a specific note id
    /// - `nullifiers_tags` similar to `note_tags`, is a list of tags used to filter the nullifiers
    /// corresponding to some notes the client is interested in
    async fn sync_state(
        &mut self,
        block_num: u32,
        account_ids: &[AccountId],
        note_tags: &[u16],
        nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, NodeRpcClientError>;
}

// STATE SYNC INFO
// ================================================================================================

/// Represents a [SyncStateResponse] with fields converted into domain types
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

/// Represents a committed note, returned as part of a [SyncStateResponse]
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

    #[allow(dead_code)]
    pub fn metadata(&self) -> NoteMetadata {
        self.metadata
    }
}

// RPC API ENDPOINT
// ================================================================================================
//
#[derive(Debug)]
pub enum RpcApiEndpoint {
    GetBlockHeaderByNumber,
    SyncState,
    SubmitProvenTx,
}

impl fmt::Display for RpcApiEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcApiEndpoint::GetBlockHeaderByNumber => write!(f, "get_block_header_by_number"),
            RpcApiEndpoint::SyncState => write!(f, "sync_state"),
            RpcApiEndpoint::SubmitProvenTx => write!(f, "submit_proven_transaction"),
        }
    }
}
