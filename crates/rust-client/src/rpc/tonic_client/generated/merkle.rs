// This file is @generated by prost-build.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MerklePath {
    #[prost(message, repeated, tag = "1")]
    pub siblings: ::prost::alloc::vec::Vec<super::digest::Digest>,
}
