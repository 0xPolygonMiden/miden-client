extern crate alloc;

mod client;
pub use client::{
    accounts::AccountTemplate, rpc, store_authenticator::StoreAuthenticator, sync::SyncSummary,
    transactions, Client, ConsumableNote, NoteRelevance,
};

pub mod config;
pub mod errors;
pub mod store;

#[cfg(all(test, feature = "executable"))]
pub mod mock;

#[cfg(all(test, feature = "executable"))]
pub mod tests;
