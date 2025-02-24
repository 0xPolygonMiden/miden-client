// This file is @generated by prost-build.
/// Represents the result of applying a block.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct ApplyBlockResponse {}
/// Represents the result of checking nullifiers.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CheckNullifiersResponse {
    /// Each requested nullifier has its corresponding nullifier proof at the same position.
    #[prost(message, repeated, tag = "1")]
    pub proofs: ::prost::alloc::vec::Vec<super::smt::SmtOpening>,
}
/// Represents the result of checking nullifiers by prefix.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CheckNullifiersByPrefixResponse {
    /// List of nullifiers matching the prefixes specified in the request.
    #[prost(message, repeated, tag = "1")]
    pub nullifiers: ::prost::alloc::vec::Vec<NullifierUpdate>,
}
/// Represents the result of getting a block header by block number.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockHeaderByNumberResponse {
    /// The requested block header.
    #[prost(message, optional, tag = "1")]
    pub block_header: ::core::option::Option<super::block::BlockHeader>,
    /// Merkle path to verify the block's inclusion in the MMR at the returned `chain_length`.
    #[prost(message, optional, tag = "2")]
    pub mmr_path: ::core::option::Option<super::merkle::MerklePath>,
    /// Current chain length.
    #[prost(fixed32, optional, tag = "3")]
    pub chain_length: ::core::option::Option<u32>,
}
/// Represents a single nullifier update.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct NullifierUpdate {
    /// Nullifier ID.
    #[prost(message, optional, tag = "1")]
    pub nullifier: ::core::option::Option<super::digest::Digest>,
    /// Block number.
    #[prost(fixed32, tag = "2")]
    pub block_num: u32,
}
/// Represents the result of syncing state request.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncStateResponse {
    /// Number of the latest block in the chain.
    #[prost(fixed32, tag = "1")]
    pub chain_tip: u32,
    /// Block header of the block with the first note matching the specified criteria.
    #[prost(message, optional, tag = "2")]
    pub block_header: ::core::option::Option<super::block::BlockHeader>,
    /// Data needed to update the partial MMR from `request.block_num + 1` to `response.block_header.block_num`.
    #[prost(message, optional, tag = "3")]
    pub mmr_delta: ::core::option::Option<super::mmr::MmrDelta>,
    /// List of account hashes updated after `request.block_num + 1` but not after `response.block_header.block_num`.
    #[prost(message, repeated, tag = "5")]
    pub accounts: ::prost::alloc::vec::Vec<super::account::AccountSummary>,
    /// List of transactions executed against requested accounts between `request.block_num + 1` and
    /// `response.block_header.block_num`.
    #[prost(message, repeated, tag = "6")]
    pub transactions: ::prost::alloc::vec::Vec<super::transaction::TransactionSummary>,
    /// List of all notes together with the Merkle paths from `response.block_header.note_root`.
    #[prost(message, repeated, tag = "7")]
    pub notes: ::prost::alloc::vec::Vec<super::note::NoteSyncRecord>,
    /// List of nullifiers created between `request.block_num + 1` and `response.block_header.block_num`.
    #[prost(message, repeated, tag = "8")]
    pub nullifiers: ::prost::alloc::vec::Vec<NullifierUpdate>,
}
/// Represents the result of syncing notes request.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SyncNoteResponse {
    /// Number of the latest block in the chain.
    #[prost(fixed32, tag = "1")]
    pub chain_tip: u32,
    /// Block header of the block with the first note matching the specified criteria.
    #[prost(message, optional, tag = "2")]
    pub block_header: ::core::option::Option<super::block::BlockHeader>,
    /// Merkle path to verify the block's inclusion in the MMR at the returned `chain_tip`.
    ///
    /// An MMR proof can be constructed for the leaf of index `block_header.block_num` of
    /// an MMR of forest `chain_tip` with this path.
    #[prost(message, optional, tag = "3")]
    pub mmr_path: ::core::option::Option<super::merkle::MerklePath>,
    /// List of all notes together with the Merkle paths from `response.block_header.note_root`.
    #[prost(message, repeated, tag = "4")]
    pub notes: ::prost::alloc::vec::Vec<super::note::NoteSyncRecord>,
}
/// An account returned as a response to the `GetBlockInputs`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AccountBlockInputRecord {
    /// The account ID.
    #[prost(message, optional, tag = "1")]
    pub account_id: ::core::option::Option<super::account::AccountId>,
    /// The latest account hash, zero hash if the account doesn't exist.
    #[prost(message, optional, tag = "2")]
    pub account_hash: ::core::option::Option<super::digest::Digest>,
    /// Merkle path to verify the account's inclusion in the MMR.
    #[prost(message, optional, tag = "3")]
    pub proof: ::core::option::Option<super::merkle::MerklePath>,
}
/// A nullifier returned as a response to the `GetBlockInputs`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NullifierBlockInputRecord {
    /// The nullifier ID.
    #[prost(message, optional, tag = "1")]
    pub nullifier: ::core::option::Option<super::digest::Digest>,
    /// Merkle path to verify the nullifier's inclusion in the MMR.
    #[prost(message, optional, tag = "2")]
    pub opening: ::core::option::Option<super::smt::SmtOpening>,
}
/// Represents the result of getting block inputs.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockInputsResponse {
    /// The latest block header.
    #[prost(message, optional, tag = "1")]
    pub block_header: ::core::option::Option<super::block::BlockHeader>,
    /// Peaks of the above block's mmr, The `forest` value is equal to the block number.
    #[prost(message, repeated, tag = "2")]
    pub mmr_peaks: ::prost::alloc::vec::Vec<super::digest::Digest>,
    /// The hashes of the requested accounts and their authentication paths.
    #[prost(message, repeated, tag = "3")]
    pub account_states: ::prost::alloc::vec::Vec<AccountBlockInputRecord>,
    /// The requested nullifiers and their authentication paths.
    #[prost(message, repeated, tag = "4")]
    pub nullifiers: ::prost::alloc::vec::Vec<NullifierBlockInputRecord>,
    /// The list of requested notes which were found in the database.
    #[prost(message, optional, tag = "5")]
    pub found_unauthenticated_notes: ::core::option::Option<
        super::note::NoteAuthenticationInfo,
    >,
}
/// Represents the result of getting batch inputs.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBatchInputsResponse {
    /// The block header that the transaction batch should reference.
    #[prost(message, optional, tag = "1")]
    pub batch_reference_block_header: ::core::option::Option<super::block::BlockHeader>,
    /// Proof of each _found_ unauthenticated note's inclusion in a block.
    #[prost(message, repeated, tag = "2")]
    pub note_proofs: ::prost::alloc::vec::Vec<super::note::NoteInclusionInBlockProof>,
    /// The serialized chain MMR which includes proofs for all blocks referenced by the
    /// above note inclusion proofs as well as proofs for inclusion of the blocks referenced
    /// by the transactions in the batch.
    #[prost(bytes = "vec", tag = "3")]
    pub chain_mmr: ::prost::alloc::vec::Vec<u8>,
}
/// An account returned as a response to the `GetTransactionInputs`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AccountTransactionInputRecord {
    /// The account ID.
    #[prost(message, optional, tag = "1")]
    pub account_id: ::core::option::Option<super::account::AccountId>,
    /// The latest account hash, zero hash if the account doesn't exist.
    #[prost(message, optional, tag = "2")]
    pub account_hash: ::core::option::Option<super::digest::Digest>,
}
/// A nullifier returned as a response to the `GetTransactionInputs`.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct NullifierTransactionInputRecord {
    /// The nullifier ID.
    #[prost(message, optional, tag = "1")]
    pub nullifier: ::core::option::Option<super::digest::Digest>,
    /// The block at which the nullifier has been consumed, zero if not consumed.
    #[prost(fixed32, tag = "2")]
    pub block_num: u32,
}
/// Represents the result of getting transaction inputs.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTransactionInputsResponse {
    /// Account state proof.
    #[prost(message, optional, tag = "1")]
    pub account_state: ::core::option::Option<AccountTransactionInputRecord>,
    /// List of nullifiers that have been consumed.
    #[prost(message, repeated, tag = "2")]
    pub nullifiers: ::prost::alloc::vec::Vec<NullifierTransactionInputRecord>,
    /// List of unauthenticated notes that were not found in the database.
    #[prost(message, repeated, tag = "3")]
    pub found_unauthenticated_notes: ::prost::alloc::vec::Vec<super::digest::Digest>,
    /// The node's current block height.
    #[prost(fixed32, tag = "4")]
    pub block_height: u32,
}
/// Represents the result of submitting proven transaction.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct SubmitProvenTransactionResponse {
    /// The node's current block height.
    #[prost(fixed32, tag = "1")]
    pub block_height: u32,
}
/// Represents the result of getting notes by IDs.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNotesByIdResponse {
    /// Lists Note's returned by the database.
    #[prost(message, repeated, tag = "1")]
    pub notes: ::prost::alloc::vec::Vec<super::note::Note>,
}
/// Represents the result of getting account details.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAccountDetailsResponse {
    /// Account info (with details for public accounts).
    #[prost(message, optional, tag = "1")]
    pub details: ::core::option::Option<super::account::AccountInfo>,
}
/// Represents the result of getting block by number.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockByNumberResponse {
    /// The requested block data encoded using \[winter_utils::Serializable\] implementation for
    /// \[miden_objects::block::Block\].
    #[prost(bytes = "vec", optional, tag = "1")]
    pub block: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
}
/// Represents the result of getting account state delta.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAccountStateDeltaResponse {
    /// The calculated account delta encoded using \[winter_utils::Serializable\] implementation
    /// for \[miden_objects::account::delta::AccountDelta\].
    #[prost(bytes = "vec", optional, tag = "1")]
    pub delta: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
}
/// Represents the result of getting account proofs.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetAccountProofsResponse {
    /// Block number at which the state of the account was returned.
    #[prost(fixed32, tag = "1")]
    pub block_num: u32,
    /// List of account state infos for the requested account keys.
    #[prost(message, repeated, tag = "2")]
    pub account_proofs: ::prost::alloc::vec::Vec<AccountProofsResponse>,
}
/// A single account proof returned as a response to the `GetAccountProofs`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AccountProofsResponse {
    /// Account ID.
    #[prost(message, optional, tag = "1")]
    pub account_id: ::core::option::Option<super::account::AccountId>,
    /// Account hash.
    #[prost(message, optional, tag = "2")]
    pub account_hash: ::core::option::Option<super::digest::Digest>,
    /// Authentication path from the `account_root` of the block header to the account.
    #[prost(message, optional, tag = "3")]
    pub account_proof: ::core::option::Option<super::merkle::MerklePath>,
    /// State header for public accounts. Filled only if `include_headers` flag is set to `true`.
    #[prost(message, optional, tag = "4")]
    pub state_header: ::core::option::Option<AccountStateHeader>,
}
/// State header for public accounts.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AccountStateHeader {
    /// Account header.
    #[prost(message, optional, tag = "1")]
    pub header: ::core::option::Option<super::account::AccountHeader>,
    /// Values of all account storage slots (max 255).
    #[prost(bytes = "vec", tag = "2")]
    pub storage_header: ::prost::alloc::vec::Vec<u8>,
    /// Account code, returned only when none of the request's code commitments match
    /// the current one.
    #[prost(bytes = "vec", optional, tag = "3")]
    pub account_code: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
    /// Storage slots information for this account
    #[prost(message, repeated, tag = "4")]
    pub storage_maps: ::prost::alloc::vec::Vec<StorageSlotMapProof>,
}
/// Represents a single storage slot with the reuqested keys and their respective values.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StorageSlotMapProof {
    /// The storage slot index (\[0..255\]).
    #[prost(uint32, tag = "1")]
    pub storage_slot: u32,
    /// Merkle proof of the map value
    #[prost(bytes = "vec", tag = "2")]
    pub smt_proof: ::prost::alloc::vec::Vec<u8>,
}
