use clap::Parser;

mod cli;
use cli::{Cli};

fn main() {
    // read command-line args
    let cli = Cli::parse();

    // execute cli action
    if let Err(error) = cli.execute() {
        println!("{}", error);
    }
}
