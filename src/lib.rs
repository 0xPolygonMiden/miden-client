extern crate alloc;

pub mod client;
pub use client::{
    accounts::AccountTemplate, rpc, store_authenticator::StoreAuthenticator, sync::SyncSummary,
    transactions, Client, NoteConsumability, NoteRelevance,
};

pub mod config;
pub mod errors;
pub mod store;

#[cfg(all(test, feature = "executable"))]
pub mod mock;

#[cfg(all(test, feature = "executable"))]
pub mod tests;
