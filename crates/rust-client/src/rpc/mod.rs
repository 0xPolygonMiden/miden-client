//! Provides an interface for the client to communicate with Miden nodes using
//! Remote Procedure Calls (RPC). It facilitates syncing with the network and submitting
//! transactions.

use alloc::{boxed::Box, collections::BTreeSet, string::String, vec::Vec};
use core::fmt;

use async_trait::async_trait;
use domain::{
    account::{AccountDetails, AccountProofs},
    note::{NetworkNote, NoteSyncInfo},
    sync::StateSyncInfo,
};
use miden_objects::{
    account::{Account, AccountCode, AccountHeader, AccountId},
    block::{BlockHeader, BlockNumber},
    crypto::merkle::MmrProof,
    note::{NoteId, NoteTag, Nullifier},
    transaction::ProvenTransaction,
};

pub mod domain;

mod errors;
pub use errors::RpcError;

mod endpoint;
pub use endpoint::Endpoint;

#[cfg(not(test))]
mod generated;
#[cfg(test)]
pub mod generated;

#[cfg(feature = "tonic")]
mod tonic_client;
#[cfg(feature = "tonic")]
pub use tonic_client::TonicRpcClient;

#[cfg(feature = "web-tonic")]
mod web_tonic_client;
#[cfg(feature = "web-tonic")]
pub use web_tonic_client::WebTonicRpcClient;

use crate::{
    store::{input_note_states::UnverifiedNoteState, InputNoteRecord},
    sync::get_nullifier_prefix,
    transaction::ForeignAccount,
};

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
    /// of the return tuple should always be Some(MmrProof).
    ///
    /// When `None` is provided, returns info regarding the latest block.
    async fn get_block_header_by_number(
        &mut self,
        block_num: Option<BlockNumber>,
        include_mmr_proof: bool,
    ) -> Result<(BlockHeader, Option<MmrProof>), RpcError>;

    /// Fetches note-related data for a list of [NoteId] using the `/GetNotesById` rpc endpoint.
    ///
    /// For any NoteType::Private note, the return data is only the
    /// [miden_objects::note::NoteMetadata], whereas for NoteType::Onchain notes, the return
    /// data includes all details.
    async fn get_notes_by_id(&mut self, note_ids: &[NoteId]) -> Result<Vec<NetworkNote>, RpcError>;

    /// Fetches info from the node necessary to perform a state sync using the
    /// `/SyncState` RPC endpoint.
    ///
    /// - `block_num` is the last block number known by the client. The returned [StateSyncInfo]
    ///   should contain data starting from the next block, until the first block which contains a
    ///   note of matching the requested tag, or the chain tip if there are no notes.
    /// - `account_ids` is a list of account IDs and determines the accounts the client is
    ///   interested in and should receive account updates of.
    /// - `note_tags` is a list of tags used to filter the notes the client is interested in, which
    ///   serves as a "note group" filter. Notice that you can't filter by a specific note ID.
    /// - `nullifiers_tags` similar to `note_tags`, is a list of tags used to filter the nullifiers
    ///   corresponding to some notes the client is interested in.
    async fn sync_state(
        &mut self,
        block_num: BlockNumber,
        account_ids: &[AccountId],
        note_tags: &[NoteTag],
        nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, RpcError>;

    /// Fetches the current state of an account from the node using the `/GetAccountDetails` RPC
    /// endpoint.
    ///
    /// - `account_id` is the ID of the wanted account.
    async fn get_account_update(
        &mut self,
        account_id: AccountId,
    ) -> Result<AccountDetails, RpcError>;

    async fn sync_notes(
        &mut self,
        block_num: BlockNumber,
        note_tags: &[NoteTag],
    ) -> Result<NoteSyncInfo, RpcError>;

    /// Fetches the nullifiers corresponding to a list of prefixes using the
    /// `/CheckNullifiersByPrefix` RPC endpoint.
    async fn check_nullifiers_by_prefix(
        &mut self,
        prefix: &[u16],
    ) -> Result<Vec<(Nullifier, u32)>, RpcError>;

    /// Fetches the account data needed to perform a Foreign Procedure Invocation (FPI) on the
    /// specified foreign accounts, using the `GetAccountProofs` endpoint.
    ///
    /// The `code_commitments` parameter is a list of known code hashes
    /// to prevent unnecessary data fetching. Returns the block number and the FPI account data. If
    /// one of the tracked accounts is not found in the node, the method will return an error.
    async fn get_account_proofs(
        &mut self,
        account_storage_requests: &BTreeSet<ForeignAccount>,
        known_account_codes: Vec<AccountCode>,
    ) -> Result<AccountProofs, RpcError>;

    /// Fetches the commit height where the nullifier was consumed. If the nullifier isn't found,
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

    /// Fetches public note-related data for a list of [NoteId] and builds [InputNoteRecord]s with
    /// it. If a note is not found or it's private, it is ignored and will not be included in the
    /// returned list.
    ///
    /// The default implementation of this method uses [NodeRpcClient::get_notes_by_id].
    async fn get_public_note_records(
        &mut self,
        note_ids: &[NoteId],
        current_timestamp: Option<u64>,
    ) -> Result<Vec<InputNoteRecord>, RpcError> {
        let note_details = self.get_notes_by_id(note_ids).await?;

        let mut public_notes = vec![];
        for detail in note_details {
            if let NetworkNote::Public(note, inclusion_proof) = detail {
                let state = UnverifiedNoteState {
                    metadata: *note.metadata(),
                    inclusion_proof,
                }
                .into();
                let note = InputNoteRecord::new(note.into(), current_timestamp, state);

                public_notes.push(note);
            }
        }

        Ok(public_notes)
    }

    /// Fetches the public accounts that have been updated since the last known state of the
    /// accounts.
    ///
    /// The `local_accounts` parameter is a list of account headers that the client has
    /// stored locally and that it wants to check for updates. If an account is private or didn't
    /// change, it is ignored and will not be included in the returned list.
    /// The default implementation of this method uses [NodeRpcClient::get_account_update].
    async fn get_updated_public_accounts(
        &mut self,
        local_accounts: &[&AccountHeader],
    ) -> Result<Vec<Account>, RpcError> {
        let mut public_accounts = vec![];

        for local_account in local_accounts {
            let response = self.get_account_update(local_account.id()).await?;

            if let AccountDetails::Public(account, _) = response {
                // We should only return an account if it's newer, otherwise we ignore it
                if account.nonce().as_int() > local_account.nonce().as_int() {
                    public_accounts.push(account);
                }
            }
        }

        Ok(public_accounts)
    }

    /// Given a block number, fetches the block header corresponding to that height from the node
    /// along with the MMR proof.
    ///
    /// The default implementation of this method uses [NodeRpcClient::get_block_header_by_number].
    async fn get_block_header_with_proof(
        &mut self,
        block_num: BlockNumber,
    ) -> Result<(BlockHeader, MmrProof), RpcError> {
        let (header, proof) = self.get_block_header_by_number(Some(block_num), true).await?;
        Ok((header, proof.ok_or(RpcError::ExpectedDataMissing(String::from("MmrProof")))?))
    }

    /// Fetches the note with the specified ID.
    ///
    /// The default implementation of this method uses [NodeRpcClient::get_notes_by_id].
    ///
    /// Errors:
    /// - [RpcError::NoteNotFound] if the note with the specified ID is not found.
    async fn get_note_by_id(&mut self, note_id: NoteId) -> Result<NetworkNote, RpcError> {
        let notes = self.get_notes_by_id(&[note_id]).await?;
        notes.into_iter().next().ok_or(RpcError::NoteNotFound(note_id))
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
