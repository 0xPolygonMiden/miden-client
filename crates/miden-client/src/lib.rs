extern crate alloc;

mod client;
pub use client::{
    accounts::AccountTemplate, rpc, store_authenticator::StoreAuthenticator, sync::SyncSummary,
    transactions, Client, NoteConsumability, NoteRelevance,
};

pub mod config;
pub mod errors;
pub mod store;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;
