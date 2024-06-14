extern crate alloc;

mod client;

pub use client::{
    accounts::AccountTemplate, get_random_coin, rpc, store_authenticator::StoreAuthenticator,
    sync::SyncSummary, transactions, Client, ConsumableNote, NoteRelevance,
};

pub mod config;
pub mod errors;
pub mod store;

#[cfg(all(test, not(feature = "wasm")))]
pub mod mock;

#[cfg(all(test, not(feature = "wasm")))]
mod tests;
