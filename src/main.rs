use clap::Parser;

mod cli;
use cli::Cli;

#[tokio::main]
async fn main() {
    // read command-line args
    let cli = Cli::parse();

    // execute cli action
    if let Err(error) = cli.execute().await {
        println!("{}", error);
    }
}
