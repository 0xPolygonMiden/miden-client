use clap::Parser;

mod cli;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt::init();
    // read command-line args
    let cli = Cli::parse();

    // execute cli action
    cli.execute().await
}
