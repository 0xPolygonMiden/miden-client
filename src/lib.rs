extern crate alloc;

pub mod client;
#[cfg(not(feature = "wasm"))]
pub mod config;
pub mod errors;
pub mod store;

#[cfg(all(any(test, feature = "test_utils"), not(feature = "wasm")))]
pub mod mock;

#[cfg(all(test, not(feature = "wasm")))]
mod tests;
