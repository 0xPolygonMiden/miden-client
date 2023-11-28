use crate::{Client, ClientConfig};
use clap::Parser;

mod account;
mod input_notes;
#[cfg(feature = "testing")]
mod test;

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

/// CLI actions
#[derive(Debug, Parser)]
pub enum Command {
    #[clap(subcommand)]
    Account(account::AccountCmd),
    #[clap(subcommand)]
    InputNotes(input_notes::InputNotes),
    #[cfg(feature = "testing")]
    /// Insert test data into the database
    TestData,
}

/// CLI entry point
impl Cli {
    pub fn execute(&self) -> Result<(), String> {
        // create a client
        let client = Client::new(ClientConfig::default()).map_err(|err| err.to_string())?;

        // execute cli command
        match &self.action {
            Command::Account(account) => account.execute(client),
            Command::InputNotes(notes) => notes.execute(client),
            #[cfg(feature = "testing")]
            Command::TestData => {
                test::insert_test_data(client);
                Ok(())
            }
        }
    }
}
