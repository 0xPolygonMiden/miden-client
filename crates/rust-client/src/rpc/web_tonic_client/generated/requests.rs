// This file is @generated by prost-build.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ApplyBlockRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub block: ::prost::alloc::vec::Vec<u8>,
}
/// Returns a list of nullifiers that match the specified prefixes and are recorded in the node.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CheckNullifiersByPrefixRequest {
    /// Number of bits used for nullifier prefix. Currently the only supported value is 16.
    #[prost(uint32, tag = "1")]
    pub prefix_len: u32,
    /// List of nullifiers to check. Each nullifier is specified by its prefix with length equal
    /// to prefix_len
    #[prost(uint32, repeated, tag = "2")]
    pub nullifiers: ::prost::alloc::vec::Vec<u32>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CheckNullifiersRequest {
    #[prost(message, repeated, tag = "1")]
    pub nullifiers: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
/// Returns the block header corresponding to the requested block number, as well as the merkle
/// path and current forest which validate the block's inclusion in the chain.
///
/// The Merkle path is an MMR proof for the block's leaf, based on the current chain length.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockHeaderByNumberRequest {
    /// The block number of the target block.
    ///
    /// If not provided, means latest know block.
    #[prost(uint32, optional, tag = "1")]
    pub block_num: ::core::option::Option<u32>,
    /// Whether or not to return authentication data for the block header.
    #[prost(bool, optional, tag = "2")]
    pub include_mmr_proof: ::core::option::Option<bool>,
}
/// State synchronization request.
///
/// Specifies state updates the client is intersted in. The server will return the first block which
/// contains a note matching `note_tags` or the chain tip. And the corresponding updates to
/// `nullifiers` and `account_ids` for that block range.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncStateRequest {
    /// Last block known by the client. The response will contain data starting from the next block,
    /// until the first block which contains a note of matching the requested tag, or the chain tip
    /// if there are no notes.
    #[prost(fixed32, tag = "1")]
    pub block_num: u32,
    /// Accounts' hash to include in the response.
    ///
    /// An account hash will be included if-and-only-if it is the latest update. Meaning it is
    /// possible there was an update to the account for the given range, but if it is not the latest,
    /// it won't be included in the response.
    #[prost(message, repeated, tag = "2")]
    pub account_ids: ::prost::alloc::vec::Vec<super::account::AccountId>,
    /// Specifies the tags which the client is interested in.
    #[prost(fixed32, repeated, tag = "3")]
    pub note_tags: ::prost::alloc::vec::Vec<u32>,
    /// Determines the nullifiers the client is interested in by specifying the 16high bits of the
    /// target nullifier.
    #[prost(uint32, repeated, tag = "4")]
    pub nullifiers: ::prost::alloc::vec::Vec<u32>,
}
/// Note synchronization request.
///
/// Specifies note tags that client is intersted in. The server will return the first block which
/// contains a note matching `note_tags` or the chain tip.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncNoteRequest {
    /// Last block known by the client. The response will contain data starting from the next block,
    /// until the first block which contains a note of matching the requested tag.
    #[prost(fixed32, tag = "1")]
    pub block_num: u32,
    /// Specifies the tags which the client is interested in.
    #[prost(fixed32, repeated, tag = "2")]
    pub note_tags: ::prost::alloc::vec::Vec<u32>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockInputsRequest {
    /// ID of the account against which a transaction is executed.
    #[prost(message, repeated, tag = "1")]
    pub account_ids: ::prost::alloc::vec::Vec<super::account::AccountId>,
    /// Array of nullifiers for all notes consumed by a transaction.
    #[prost(message, repeated, tag = "2")]
    pub nullifiers: ::prost::alloc::vec::Vec<super::digest::Digest>,
    /// Array of note IDs to be checked for existence in the database.
    #[prost(message, repeated, tag = "3")]
    pub unauthenticated_notes: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTransactionInputsRequest {
    #[prost(message, optional, tag = "1")]
    pub account_id: ::core::option::Option<super::account::AccountId>,
    #[prost(message, repeated, tag = "2")]
    pub nullifiers: ::prost::alloc::vec::Vec<super::digest::Digest>,
    #[prost(message, repeated, tag = "3")]
    pub unauthenticated_notes: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubmitProvenTransactionRequest {
    /// Transaction encoded using miden's native format
    #[prost(bytes = "vec", tag = "1")]
    pub transaction: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNotesByIdRequest {
    /// List of NoteId's to be queried from the database
    #[prost(message, repeated, tag = "1")]
    pub note_ids: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNoteAuthenticationInfoRequest {
    /// List of NoteId's to be queried from the database
    #[prost(message, repeated, tag = "1")]
    pub note_ids: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListNullifiersRequest {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListAccountsRequest {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ListNotesRequest {}
/// Returns the latest state of an account with the specified ID.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAccountDetailsRequest {
    /// Account ID to get details.
    #[prost(message, optional, tag = "1")]
    pub account_id: ::core::option::Option<super::account::AccountId>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockByNumberRequest {
    /// The block number of the target block.
    #[prost(fixed32, tag = "1")]
    pub block_num: u32,
}
/// Returns delta of the account states in the range from `from_block_num` (exclusive) to
/// `to_block_num` (inclusive).
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAccountStateDeltaRequest {
    /// ID of the account for which the delta is requested.
    #[prost(message, optional, tag = "1")]
    pub account_id: ::core::option::Option<super::account::AccountId>,
    /// Block number from which the delta is requested (exclusive).
    #[prost(fixed32, tag = "2")]
    pub from_block_num: u32,
    /// Block number up to which the delta is requested (inclusive).
    #[prost(fixed32, tag = "3")]
    pub to_block_num: u32,
}
