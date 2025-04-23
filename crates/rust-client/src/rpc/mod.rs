//! Provides an interface for the client to communicate with a Miden node using
//! Remote Procedure Calls (RPC).
//!
//! This module defines the [`NodeRpcClient`] trait which abstracts calls to the RPC protocol used
//! to:
//!
//! - Submit proven transactions.
//! - Retrieve block headers (optionally with MMR proofs).
//! - Sync state updates (including notes, nullifiers, and account updates).
//! - Fetch details for specific notes and accounts.
//!
//! In addition, the module provides implementations for different environments (e.g. tonic-based or
//! web-based) via feature flags ( `tonic` and `web-tonic`).
//!
//! ## Example
//!
//! ```no_run
//! # use miden_client::rpc::{Endpoint, NodeRpcClient, TonicRpcClient};
//! # use miden_objects::block::BlockNumber;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a Tonic RPC client instance (assumes default endpoint configuration).
//! let endpoint = Endpoint::new("https".into(), "localhost".into(), Some(57291));
//! let mut rpc_client = TonicRpcClient::new(&endpoint, 1000);
//!
//! // Fetch the latest block header (by passing None).
//! let (block_header, mmr_proof) = rpc_client.get_block_header_by_number(None, true).await?;
//!
//! println!("Latest block number: {}", block_header.block_num());
//! if let Some(proof) = mmr_proof {
//!     println!("MMR proof received accordingly");
//! }
//!
//! #    Ok(())
//! # }
//! ```
//! The client also makes use of this component in order to communicate with the node.
//!
//! For further details and examples, see the documentation for the individual methods in the
//! [`NodeRpcClient`] trait.

use alloc::{boxed::Box, collections::BTreeSet, string::String, vec::Vec};
use core::fmt;

use async_trait::async_trait;
use domain::{
    account::{AccountDetails, AccountProofs},
    note::{NetworkNote, NoteSyncInfo},
    nullifier::NullifierUpdate,
    sync::StateSyncInfo,
};
use miden_objects::{
    account::{Account, AccountCode, AccountDelta, AccountHeader, AccountId},
    block::{BlockHeader, BlockNumber, ProvenBlock},
    crypto::merkle::{MmrProof, SmtProof},
    note::{NoteId, NoteTag, Nullifier},
    transaction::ProvenTransaction,
};

/// Contains domain types related to RPC requests and responses, as well as utility functions
/// for dealing with them.
pub mod domain;

mod errors;
pub use errors::RpcError;

mod endpoint;
pub use endpoint::Endpoint;

#[cfg(not(feature = "testing"))]
mod generated;
#[cfg(feature = "testing")]
pub mod generated;

#[cfg(all(feature = "tonic", feature = "web-tonic"))]
compile_error!("features `tonic` and `web-tonic` are mutually exclusive");

#[cfg(any(feature = "tonic", feature = "web-tonic"))]
mod tonic_client;
#[cfg(any(feature = "tonic", feature = "web-tonic"))]
pub use tonic_client::TonicRpcClient;

use crate::{
    store::{InputNoteRecord, input_note_states::UnverifiedNoteState},
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
        &self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), RpcError>;

    /// Given a block number, fetches the block header corresponding to that height from the node
    /// using the `/GetBlockHeaderByNumber` endpoint.
    /// If `include_mmr_proof` is set to true and the function returns an `Ok`, the second value
    /// of the return tuple should always be Some(MmrProof).
    ///
    /// When `None` is provided, returns info regarding the latest block.
    async fn get_block_header_by_number(
        &self,
        block_num: Option<BlockNumber>,
        include_mmr_proof: bool,
    ) -> Result<(BlockHeader, Option<MmrProof>), RpcError>;

    /// Given a block number, fetches the block corresponding to that height from the node using
    /// the `/GetBlockByNumber` RPC endpoint.
    async fn get_block_by_number(&self, block_num: BlockNumber) -> Result<ProvenBlock, RpcError>;

    /// Fetches note-related data for a list of [NoteId] using the `/GetNotesById` rpc endpoint.
    ///
    /// For any NoteType::Private note, the return data is only the
    /// [miden_objects::note::NoteMetadata], whereas for NoteType::Onchain notes, the return
    /// data includes all details.
    async fn get_notes_by_id(&self, note_ids: &[NoteId]) -> Result<Vec<NetworkNote>, RpcError>;

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
        &self,
        block_num: BlockNumber,
        account_ids: &[AccountId],
        note_tags: &[NoteTag],
    ) -> Result<StateSyncInfo, RpcError>;

    /// Fetches the current state of an account from the node using the `/GetAccountDetails` RPC
    /// endpoint.
    ///
    /// - `account_id` is the ID of the wanted account.
    async fn get_account_details(&self, account_id: AccountId) -> Result<AccountDetails, RpcError>;

    /// Fetches the notes related to the specified tags using the `/SyncNotes` RPC endpoint.
    ///
    /// - `block_num` is the last block number known by the client.
    /// - `note_tags` is a list of tags used to filter the notes the client is interested in.
    async fn sync_notes(
        &self,
        block_num: BlockNumber,
        note_tags: &[NoteTag],
    ) -> Result<NoteSyncInfo, RpcError>;

    /// Fetches the nullifiers corresponding to a list of prefixes using the
    /// `/CheckNullifiersByPrefix` RPC endpoint.
    ///
    /// - `prefix` is a list of nullifiers prefixes to search for.
    /// - `block_num` is the block number to start the search from. Nullifiers created in this block
    ///   or the following blocks will be included.
    async fn check_nullifiers_by_prefix(
        &self,
        prefix: &[u16],
        block_num: BlockNumber,
    ) -> Result<Vec<NullifierUpdate>, RpcError>;

    /// Fetches the nullifier proofs corresponding to a list of nullifiers using the
    /// `/CheckNullifiers` RPC endpoint.
    async fn check_nullifiers(&self, nullifiers: &[Nullifier]) -> Result<Vec<SmtProof>, RpcError>;

    /// Fetches the account data needed to perform a Foreign Procedure Invocation (FPI) on the
    /// specified foreign accounts, using the `GetAccountProofs` endpoint.
    ///
    /// The `code_commitments` parameter is a list of known code commitments
    /// to prevent unnecessary data fetching. Returns the block number and the FPI account data. If
    /// one of the tracked accounts is not found in the node, the method will return an error.
    async fn get_account_proofs(
        &self,
        account_storage_requests: &BTreeSet<ForeignAccount>,
        known_account_codes: Vec<AccountCode>,
    ) -> Result<AccountProofs, RpcError>;

    /// Fetches the account state delta for the specified account between the specified blocks
    /// using the `/GetAccountStateDelta` RPC endpoint.
    async fn get_account_state_delta(
        &self,
        account_id: AccountId,
        from_block: BlockNumber,
        to_block: BlockNumber,
    ) -> Result<AccountDelta, RpcError>;

    /// Fetches the commit height where the nullifier was consumed. If the nullifier isn't found,
    /// then `None` is returned.
    /// The `block_num` parameter is the block number to start the search from.
    ///
    /// The default implementation of this method uses [NodeRpcClient::check_nullifiers_by_prefix].
    async fn get_nullifier_commit_height(
        &self,
        nullifier: &Nullifier,
        block_num: BlockNumber,
    ) -> Result<Option<u32>, RpcError> {
        let nullifiers = self.check_nullifiers_by_prefix(&[nullifier.prefix()], block_num).await?;

        Ok(nullifiers
            .iter()
            .find(|update| update.nullifier == *nullifier)
            .map(|update| update.block_num))
    }

    /// Fetches public note-related data for a list of [NoteId] and builds [InputNoteRecord]s with
    /// it. If a note is not found or it's private, it is ignored and will not be included in the
    /// returned list.
    ///
    /// The default implementation of this method uses [NodeRpcClient::get_notes_by_id].
    async fn get_public_note_records(
        &self,
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
    /// The default implementation of this method uses [NodeRpcClient::get_account_details].
    async fn get_updated_public_accounts(
        &self,
        local_accounts: &[&AccountHeader],
    ) -> Result<Vec<Account>, RpcError> {
        let mut public_accounts = vec![];

        for local_account in local_accounts {
            let response = self.get_account_details(local_account.id()).await?;

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
        &self,
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
    async fn get_note_by_id(&self, note_id: NoteId) -> Result<NetworkNote, RpcError> {
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
    CheckNullifiers,
    CheckNullifiersByPrefix,
    GetAccountDetails,
    GetAccountStateDelta,
    GetAccountProofs,
    GetBlockByNumber,
    GetBlockHeaderByNumber,
    SyncState,
    SubmitProvenTx,
    SyncNotes,
}

impl fmt::Display for NodeRpcClientEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeRpcClientEndpoint::CheckNullifiers => write!(f, "check_nullifiers"),
            NodeRpcClientEndpoint::CheckNullifiersByPrefix => {
                write!(f, "check_nullifiers_by_prefix")
            },
            NodeRpcClientEndpoint::GetAccountDetails => write!(f, "get_account_details"),
            NodeRpcClientEndpoint::GetAccountStateDelta => write!(f, "get_account_state_delta"),
            NodeRpcClientEndpoint::GetAccountProofs => write!(f, "get_account_proofs"),
            NodeRpcClientEndpoint::GetBlockByNumber => write!(f, "get_block_by_number"),
            NodeRpcClientEndpoint::GetBlockHeaderByNumber => {
                write!(f, "get_block_header_by_number")
            },
            NodeRpcClientEndpoint::SyncState => write!(f, "sync_state"),
            NodeRpcClientEndpoint::SubmitProvenTx => write!(f, "submit_proven_transaction"),
            NodeRpcClientEndpoint::SyncNotes => write!(f, "sync_notes"),
        }
    }
}
