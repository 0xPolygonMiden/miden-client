use clap::Parser;
use miden_client::{Client, ClientConfig};

mod cli;
use cli::Cli;

fn main() {
    // read command-line args
    let cli = Cli::parse();

    // execute cli action
    if let Err(error) = cli.execute() {
        println!("{}", error);
    }
}
