#[cfg(not(feature = "wasm"))]
use clap::Parser;

extern crate alloc;
mod cli;

#[cfg(not(feature = "wasm"))]
use cli::Cli;

#[cfg(not(feature = "wasm"))]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    // read command-line args
    let cli = Cli::parse();

    // execute cli action
    if let Err(error) = cli.execute().await {
        println!("{}", error);
    }
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
