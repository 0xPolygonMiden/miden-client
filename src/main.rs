#[cfg(not(feature = "wasm"))]
use clap::Parser;

extern crate alloc;
mod cli;

#[cfg(not(feature = "wasm"))]
use cli::Cli;

#[cfg(not(feature = "wasm"))]
#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt::init();
    // read command-line args
    let cli = Cli::parse();

    // execute cli action
    cli.execute().await
}

#[cfg(feature = "wasm")]
pub mod client;

#[cfg(feature = "wasm")]
pub mod store;

#[cfg(feature = "wasm")]
pub mod errors;

#[cfg(feature = "wasm")]
fn main() {
}
