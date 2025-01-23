// This file is @generated by prost-build.
/// Represents a note's metadata.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NoteMetadata {
    /// The account which sent the note.
    #[prost(message, optional, tag = "1")]
    pub sender: ::core::option::Option<super::account::AccountId>,
    /// The type of the note (0b01 = public, 0b10 = private, 0b11 = encrypted).
    #[prost(uint32, tag = "2")]
    pub note_type: u32,
    /// A value which can be used by the recipient(s) to identify notes intended for them.
    ///
    /// See `miden_objects::note::note_tag` for more info.
    #[prost(fixed32, tag = "3")]
    pub tag: u32,
    /// Specifies when a note is ready to be consumed.
    ///
    /// See `miden_objects::note::execution_hint` for more info.
    #[prost(fixed64, tag = "4")]
    pub execution_hint: u64,
    /// An arbitrary user-defined value.
    #[prost(fixed64, tag = "5")]
    pub aux: u64,
}
/// Represents a note.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Note {
    /// The block number in which the note was created.
    #[prost(fixed32, tag = "1")]
    pub block_num: u32,
    /// The index of the note in the block.
    #[prost(uint32, tag = "2")]
    pub note_index: u32,
    /// The ID of the note.
    #[prost(message, optional, tag = "3")]
    pub note_id: ::core::option::Option<super::digest::Digest>,
    /// The note's metadata.
    #[prost(message, optional, tag = "4")]
    pub metadata: ::core::option::Option<NoteMetadata>,
    /// The note's inclusion proof in the block.
    #[prost(message, optional, tag = "5")]
    pub merkle_path: ::core::option::Option<super::merkle::MerklePath>,
    /// Serialized details of the public note (empty for private notes).
    #[prost(bytes = "vec", optional, tag = "6")]
    pub details: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
}
/// Represents a proof of note's inclusion in a block.
///
/// Does not include proof of the block's inclusion in the chain.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NoteInclusionInBlockProof {
    /// A unique identifier of the note which is a 32-byte commitment to the underlying note data.
    #[prost(message, optional, tag = "1")]
    pub note_id: ::core::option::Option<super::digest::Digest>,
    /// The block number in which the note was created.
    #[prost(fixed32, tag = "2")]
    pub block_num: u32,
    /// The index of the note in the block.
    #[prost(uint32, tag = "3")]
    pub note_index_in_block: u32,
    /// The note's inclusion proof in the block.
    #[prost(message, optional, tag = "4")]
    pub merkle_path: ::core::option::Option<super::merkle::MerklePath>,
}
/// Represents proof of a note inclusion in the block.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NoteSyncRecord {
    /// The index of the note.
    #[prost(uint32, tag = "1")]
    pub note_index: u32,
    /// A unique identifier of the note which is a 32-byte commitment to the underlying note data.
    #[prost(message, optional, tag = "2")]
    pub note_id: ::core::option::Option<super::digest::Digest>,
    /// The note's metadata.
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<NoteMetadata>,
    /// The note's inclusion proof in the block.
    #[prost(message, optional, tag = "4")]
    pub merkle_path: ::core::option::Option<super::merkle::MerklePath>,
}
/// Represents proof of notes inclusion in the block(s) and block(s) inclusion in the chain.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NoteAuthenticationInfo {
    /// Proof of each note's inclusion in a block.
    #[prost(message, repeated, tag = "1")]
    pub note_proofs: ::prost::alloc::vec::Vec<NoteInclusionInBlockProof>,
    /// Proof of each block's inclusion in the chain.
    #[prost(message, repeated, tag = "2")]
    pub block_proofs: ::prost::alloc::vec::Vec<super::block::BlockInclusionProof>,
}
