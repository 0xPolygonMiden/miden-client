// This file is @generated by prost-build.
/// Represents an MMR delta.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MmrDelta {
    /// The number of leaf nodes in the MMR.
    #[prost(uint64, tag = "1")]
    pub forest: u64,
    /// New and changed MMR peaks.
    #[prost(message, repeated, tag = "2")]
    pub data: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
