//! Provides an interface for the client to communicate with Miden nodes using
//! Remote Procedure Calls (RPC). It facilitates syncing with the network and submitting
//! transactions.

use alloc::{boxed::Box, collections::BTreeSet, vec::Vec};
use core::fmt;

use async_trait::async_trait;
use domain::{
    accounts::AccountProofs,
    notes::{AccountDetails, NoteDetails, NoteSyncInfo},
    state::StateSyncInfo,
};

mod errors;
pub(crate) use errors::RpcConversionError;
pub use errors::RpcError;
use miden_objects::{
    accounts::AccountId,
    crypto::merkle::MmrProof,
    notes::{NoteId, NoteTag, Nullifier},
    transaction::ProvenTransaction,
    BlockHeader, Digest,
};

#[cfg(all(feature = "tonic", feature = "web-tonic"))]
compile_error!("features `tonic` and `web-tonic` are mutually exclusive");

pub mod domain;

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
    /// For any NoteType::Private note, the return data is only the
    /// [miden_objects::notes::NoteMetadata], whereas for NoteType::Onchain notes, the return
    /// data includes all details.
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
        account_ids: &BTreeSet<AccountId>,
        code_commitments: &[Digest],
        include_headers: bool,
    ) -> Result<AccountProofs, RpcError>;

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
