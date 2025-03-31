use miden_cli::Cli;

extern crate std;

#[tokio::main]
async fn main() -> miette::Result<()> {
    use clap::Parser;

    tracing_subscriber::fmt::init();
    // read command-line args
    println!("ABOUT TO PARSE");
    let cli = Cli::parse();

    println!("PARSED");
    // execute cli action
    Ok(cli.execute().await?)
}
