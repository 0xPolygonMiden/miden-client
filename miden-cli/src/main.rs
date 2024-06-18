extern crate alloc;

mod cli;

#[tokio::main]
async fn main() -> Result<(), String> {
    use clap::Parser;

    tracing_subscriber::fmt::init();
    // read command-line args
    let cli = cli::Cli::parse();

    // execute cli action
    cli.execute().await
}
