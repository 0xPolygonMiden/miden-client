use clap::Parser;

mod account;

/// CLI actions
#[derive(Debug, Parser)]
pub enum Command {
    #[clap(subcommand)]
    Account(account::AccountCmd),
}

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(
    name = "Miden",
    about = "Miden Client",
    version,
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[clap(subcommand)]
    action: Command,
}

/// CLI entry point
impl Cli {
    pub fn execute(&self) -> Result<(), String> {
        match &self.action {
            Command::Account(account) => account.execute(),
        }
    }
}
