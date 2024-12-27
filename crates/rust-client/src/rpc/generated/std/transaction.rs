// This file is @generated by prost-build.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct TransactionId {
    #[prost(message, optional, tag = "1")]
    pub id: ::core::option::Option<super::digest::Digest>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionSummary {
    #[prost(message, optional, tag = "1")]
    pub transaction_id: ::core::option::Option<TransactionId>,
    #[prost(fixed32, tag = "2")]
    pub block_num: u32,
    #[prost(message, optional, tag = "3")]
    pub account_id: ::core::option::Option<super::account::AccountId>,
}
