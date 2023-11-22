use clap::Parser;
use miden_client::{Client, ClientConfig};

// ACCOUNT COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "View accounts and account details")]
pub struct AccountCmd {
    /// List all accounts monitored by this client
    #[clap(short = 'l', long = "list")]
    list: bool,

    /// View details of the account for the specified ID
    #[clap(short = 'v', long = "view", value_name = "ID")]
    view: Option<String>,
}

impl AccountCmd {
    pub fn execute(&self) -> Result<(), String> {
        println!("list: {}", self.list);
        println!("view: {:?}", self.view);
        list_accounts();
        Ok(())
    }
}

// LIST ACCOUNTS
// ================================================================================================

pub fn list_accounts() {
    println!("{}", "-".repeat(240));
    println!(
        "{0: <18} | {1: <66} | {2: <66} | {3: <66} | {4: <15}",
        "account id", "code root", "vault root", "storage root", "nonce",
    );
    println!("{}", "-".repeat(240));

    let client = Client::new(ClientConfig::default()).unwrap();
    let accounts = client.get_accounts().unwrap();

    for acct in accounts {
        println!(
            "{0: <18} | {1: <66} | {2: <66} | {3: <66} | {4: <15}",
            acct.id(),
            acct.code_root(),
            acct.vault_root(),
            acct.storage_root(),
            acct.nonce(),
        );
    }
    println!("{}", "-".repeat(240));
}
