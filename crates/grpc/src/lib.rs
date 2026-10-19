//! Default gRPC implementation of the `miden-client-core` `NodeRpcClient` trait.

#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod conversions;
mod grpc_support;
mod node_errors;

mod generated;

mod tonic_client;
pub use tonic_client::GrpcClient;
