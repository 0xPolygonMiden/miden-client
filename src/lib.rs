extern crate alloc;

mod client;

pub use client::{
    accounts::AccountTemplate, store_authenticator::StoreAuthenticator, Client, ConsumableNote,
    NoteRelevance,
};

pub mod rpc {
    pub use super::client::rpc::*;
}

pub mod state_sync {
    pub use super::client::sync::SyncSummary;
}

pub mod transactions {
    pub use super::client::transactions::*;
}

pub mod utils {
    pub use super::client::get_random_coin;
}

pub mod config;
pub mod errors;
pub mod store;

#[cfg(any(test, feature = "test_utils"))]
pub mod mock;

#[cfg(test)]
mod tests;
