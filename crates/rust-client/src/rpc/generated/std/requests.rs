// This file is @generated by prost-build.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ApplyBlockRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub block: ::prost::alloc::vec::Vec<u8>,
}
/// Returns a list of nullifiers that match the specified prefixes and are recorded in the node.
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
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CheckNullifiersRequest {
    #[prost(message, repeated, tag = "1")]
    pub nullifiers: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
/// Returns the block header corresponding to the requested block number, as well as the merkle
/// path and current forest which validate the block's inclusion in the chain.
///
/// The Merkle path is an MMR proof for the block's leaf, based on the current chain length.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct GetBlockHeaderByNumberRequest {
    /// The block number of the target block.
    ///
    /// If not provided, means latest known block.
    #[prost(uint32, optional, tag = "1")]
    pub block_num: ::core::option::Option<u32>,
    /// Whether or not to return authentication data for the block header.
    #[prost(bool, optional, tag = "2")]
    pub include_mmr_proof: ::core::option::Option<bool>,
}
/// State synchronization request.
///
/// Specifies state updates the client is interested in. The server will return the first block which
/// contains a note matching `note_tags` or the chain tip. And the corresponding updates to
/// `nullifiers` and `account_ids` for that block range.
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
/// Specifies note tags that client is interested in. The server will return the first block which
/// contains a note matching `note_tags` or the chain tip.
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
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTransactionInputsRequest {
    #[prost(message, optional, tag = "1")]
    pub account_id: ::core::option::Option<super::account::AccountId>,
    #[prost(message, repeated, tag = "2")]
    pub nullifiers: ::prost::alloc::vec::Vec<super::digest::Digest>,
    #[prost(message, repeated, tag = "3")]
    pub unauthenticated_notes: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubmitProvenTransactionRequest {
    /// Transaction encoded using miden's native format
    #[prost(bytes = "vec", tag = "1")]
    pub transaction: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNotesByIdRequest {
    /// List of NoteId's to be queried from the database
    #[prost(message, repeated, tag = "1")]
    pub note_ids: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNoteAuthenticationInfoRequest {
    /// List of NoteId's to be queried from the database
    #[prost(message, repeated, tag = "1")]
    pub note_ids: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct ListNullifiersRequest {}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct ListAccountsRequest {}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct ListNotesRequest {}
/// Returns the latest state of an account with the specified ID.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct GetAccountDetailsRequest {
    /// Account ID to get details.
    #[prost(message, optional, tag = "1")]
    pub account_id: ::core::option::Option<super::account::AccountId>,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct GetBlockByNumberRequest {
    /// The block number of the target block.
    #[prost(fixed32, tag = "1")]
    pub block_num: u32,
}
/// Returns delta of the account states in the range from `from_block_num` (exclusive) to
/// `to_block_num` (inclusive).
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
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
/// Request message to get account proofs.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAccountProofsRequest {
    /// A list of account requests, including map keys + values.
    #[prost(message, repeated, tag = "1")]
    pub account_requests: ::prost::alloc::vec::Vec<
        get_account_proofs_request::AccountRequest,
    >,
    /// Optional flag to include header and account code in the response. False by default.
    #[prost(bool, optional, tag = "2")]
    pub include_headers: ::core::option::Option<bool>,
    /// Account code commitments corresponding to the last-known `AccountCode` for requested
    /// accounts. Responses will include only the ones that are not known to the caller.
    /// These are not associated with a specific account but rather, they will be matched against
    /// all requested accounts.
    #[prost(message, repeated, tag = "3")]
    pub code_commitments: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
/// Nested message and enum types in `GetAccountProofsRequest`.
pub mod get_account_proofs_request {
    /// Represents per-account requests where each account ID has its own list of
    /// (storage_slot_index, map_keys) pairs.
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AccountRequest {
        /// The account ID for this request.
        #[prost(message, optional, tag = "1")]
        pub account_id: ::core::option::Option<super::super::account::AccountId>,
        /// List of storage requests for this account.
        #[prost(message, repeated, tag = "2")]
        pub storage_requests: ::prost::alloc::vec::Vec<StorageRequest>,
    }
    /// Represents a storage slot index and the associated map keys.
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct StorageRequest {
        /// Storage slot index (\[0..255\])
        #[prost(uint32, tag = "1")]
        pub storage_slot_index: u32,
        /// A list of map keys (Digests) associated with this storage slot.
        #[prost(message, repeated, tag = "2")]
        pub map_keys: ::prost::alloc::vec::Vec<super::super::digest::Digest>,
    }
}
