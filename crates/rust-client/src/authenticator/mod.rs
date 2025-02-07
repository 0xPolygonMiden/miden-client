pub use miden_tx::AuthenticationError;

#[cfg(not(target_arch = "wasm32"))]
mod client_authenticator;
#[cfg(not(target_arch = "wasm32"))]
pub use client_authenticator::ClientAuthenticator;

#[cfg(target_arch = "wasm32")]
mod web_authenticator;
#[cfg(target_arch = "wasm32")]
pub use web_authenticator::WebAuthenticator;
