#[cfg(feature = "executable")]
use clap::Parser;

extern crate alloc;
#[cfg(feature = "executable")]
mod cli;

#[tokio::main]
async fn main() -> Result<(), String> {
    #[cfg(feature = "executable")]
    {
        tracing_subscriber::fmt::init();
        // read command-line args
        let cli = cli::Cli::parse();

        // execute cli action
        cli.execute().await
    }
}
