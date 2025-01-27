use miden_cli::Cli;

extern crate std;

#[tokio::main]
async fn main() -> miette::Result<()> {
    use clap::Parser;

    tracing_subscriber::fmt::init();
    // read command-line args
    let cli = Cli::parse();

    // execute cli action
    Ok(cli.execute().await?)
}
