use std::io;

use clap::{Parser, ValueEnum};
use miden_client::{
    client::{
        rpc::NodeRpcClient,
        transactions::{
            transaction_request::{PaymentTransactionData, TransactionTemplate},
            TransactionResult,
        },
    },
    store::Store,
};
use miden_objects::{
    assets::{Asset, FungibleAsset},
    crypto::rand::FeltRng,
    notes::{NoteId, NoteType as MidenNoteType},
    Digest,
};
use miden_tx::TransactionAuthenticator;

use super::{get_input_note_with_id_prefix, parse_account_id, Client};
use crate::cli::create_dynamic_table;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum NoteType {
    Public,
    Private,
}

impl From<&NoteType> for MidenNoteType {
    fn from(note_type: &NoteType) -> Self {
        match note_type {
            NoteType::Public => MidenNoteType::Public,
            NoteType::Private => MidenNoteType::OffChain,
        }
    }
}

#[derive(Debug, Parser, Clone)]
/// Mint tokens from a fungible faucet to a wallet.
pub struct MintCmd {
    /// Target account ID or its hex prefix
    #[clap(short = 't', long = "target")]
    target_account_id: String,
    /// Faucet account ID or its hex prefix
    #[clap(short = 'f', long = "faucet")]
    faucet_id: String,
    /// Amount of tokens to mint
    #[clap(short, long)]
    amount: u64,
    #[clap(short, long, value_enum)]
    note_type: NoteType,
    /// Flag to submit the executed transaction without asking for confirmation
    #[clap(short, long, default_value_t = false)]
    force: bool,
}

impl MintCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        self,
        mut client: Client<N, R, S, A>,
        default_account_id: Option<String>,
    ) -> Result<(), String> {
        let force = self.force;
        let transaction_template = self.into_template(&client, default_account_id)?;
        execute_transaction(&mut client, transaction_template, force).await
    }

    fn into_template<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        self,
        client: &Client<N, R, S, A>,
        _default_account_id: Option<String>,
    ) -> Result<TransactionTemplate, String> {
        let faucet_id = parse_account_id(client, self.faucet_id.as_str())?;
        let fungible_asset =
            FungibleAsset::new(faucet_id, self.amount).map_err(|err| err.to_string())?;
        let target_account_id = parse_account_id(client, self.target_account_id.as_str())?;

        Ok(TransactionTemplate::MintFungibleAsset(
            fungible_asset,
            target_account_id,
            (&self.note_type).into(),
        ))
    }
}

#[derive(Debug, Parser, Clone)]
/// Create a pay-to-id transaction.
pub struct SendCmd {
    /// Sender account ID or its hex prefix. If none is provided, the default account's ID is used instead
    #[clap(short = 's', long = "sender")]
    sender_account_id: Option<String>,
    /// Target account ID or its hex prefix
    #[clap(short = 't', long = "target")]
    target_account_id: String,
    /// Faucet account ID or its hex prefix
    #[clap(short = 'f', long = "faucet")]
    faucet_id: String,
    #[clap(short, long, value_enum)]
    note_type: NoteType,
    /// Flag to submit the executed transaction without asking for confirmation
    #[clap(long, default_value_t = false)]
    force: bool,
    /// Set the recall height for the transaction. If the note was not consumed by this height, the sender may consume it back.
    ///
    /// Setting this flag turns the transaction from a PayToId to a PayToIdWithRecall.
    #[clap(short, long)]
    recall_height: Option<u32>,
    /// Amount of tokens to mint
    amount: u64,
}

impl SendCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        self,
        mut client: Client<N, R, S, A>,
        default_account_id: Option<String>,
    ) -> Result<(), String> {
        let force = self.force;
        let transaction_template = self.into_template(&client, default_account_id)?;
        execute_transaction(&mut client, transaction_template, force).await
    }

    fn into_template<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        self,
        client: &Client<N, R, S, A>,
        default_account_id: Option<String>,
    ) -> Result<TransactionTemplate, String> {
        let faucet_id = parse_account_id(client, self.faucet_id.as_str())?;
        let fungible_asset = FungibleAsset::new(faucet_id, self.amount)
            .map_err(|err| err.to_string())?
            .into();

        // try to use either the provided argument or the default account
        let sender_account_id = self
            .sender_account_id
            .clone()
            .or(default_account_id)
            .ok_or("Neither a sender nor a default account was provided".to_string())?;
        let sender_account_id = parse_account_id(client, &sender_account_id)?;
        let target_account_id = parse_account_id(client, self.target_account_id.as_str())?;

        let payment_transaction =
            PaymentTransactionData::new(fungible_asset, sender_account_id, target_account_id);
        if let Some(recall_height) = self.recall_height {
            Ok(TransactionTemplate::PayToIdWithRecall(
                payment_transaction,
                recall_height,
                (&self.note_type).into(),
            ))
        } else {
            Ok(TransactionTemplate::PayToId(payment_transaction, (&self.note_type).into()))
        }
    }
}

#[derive(Debug, Parser, Clone)]
/// Consume with the account corresponding to `account_id` all of the notes from `list_of_notes`.
pub struct ConsumeNotesCmd {
    /// The account ID to be used to consume the note or its hex prefix. If none is provided, the default
    /// account's ID is used instead
    #[clap(short = 'a', long = "account")]
    account_id: Option<String>,
    /// A list of note IDs or the hex prefixes of their corresponding IDs
    list_of_notes: Vec<String>,
    /// Flag to submit the executed transaction without asking for confirmation
    #[clap(short, long, default_value_t = false)]
    force: bool,
}

impl ConsumeNotesCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        self,
        mut client: Client<N, R, S, A>,
        default_account_id: Option<String>,
    ) -> Result<(), String> {
        let force = self.force;
        let transaction_template = self.into_template(&client, default_account_id)?;
        execute_transaction(&mut client, transaction_template, force).await
    }

    fn into_template<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        self,
        client: &Client<N, R, S, A>,
        default_account_id: Option<String>,
    ) -> Result<TransactionTemplate, String> {
        let list_of_notes = self
            .list_of_notes
            .iter()
            .map(|note_id| {
                get_input_note_with_id_prefix(client, note_id)
                    .map(|note_record| note_record.id())
                    .map_err(|err| err.to_string())
            })
            .collect::<Result<Vec<NoteId>, _>>()?;

        let account_id = self
            .account_id
            .clone()
            .or(default_account_id)
            .ok_or("Neither a sender nor a default account was provided".to_string())?;
        let account_id = parse_account_id(client, &account_id)?;

        Ok(TransactionTemplate::ConsumeNotes(account_id, list_of_notes))
    }
}

// EXECUTE TRANSACTION
// ================================================================================================
async fn execute_transaction<
    N: NodeRpcClient,
    R: FeltRng,
    S: Store,
    A: TransactionAuthenticator,
>(
    client: &mut Client<N, R, S, A>,
    transaction_template: TransactionTemplate,
    force: bool,
) -> Result<(), String> {
    let transaction_request = client.build_transaction_request(transaction_template)?;

    println!("Executing transaction...");
    let transaction_execution_result = client.new_transaction(transaction_request)?;

    // Show delta and ask for confirmation
    print_transaction_details(&transaction_execution_result);
    if !force {
        println!("Continue with proving and submission? Changes will be irreversible once the proof is finalized on the rollup (Y/N)");
        let mut proceed_str: String = String::new();
        io::stdin().read_line(&mut proceed_str).expect("Should read line");

        if proceed_str.trim().to_lowercase() != "y" {
            println!("Transaction was cancelled.");
            return Ok(());
        }
    }

    println!("Proving transaction and then submitting it to node...");

    let transaction_id = transaction_execution_result.executed_transaction().id();
    let output_notes = transaction_execution_result
        .created_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();
    client.submit_transaction(transaction_execution_result).await?;

    println!("Succesfully created transaction.");
    println!("Transaction ID: {}", transaction_id);
    println!("Output notes:");
    output_notes.iter().for_each(|note_id| println!("\t- {}", note_id));

    Ok(())
}

fn print_transaction_details(transaction_result: &TransactionResult) {
    println!(
        "The transaction will have the following effects on the account with ID {}",
        transaction_result.executed_transaction().account_id()
    );

    let account_delta = transaction_result.account_delta();
    let mut table = create_dynamic_table(&["Storage Slot", "Effect"]);

    for cleared_item_slot in account_delta.storage().cleared_items.iter() {
        table.add_row(vec![cleared_item_slot.to_string(), "Cleared".to_string()]);
    }

    for (updated_item_slot, new_value) in account_delta.storage().updated_items.iter() {
        let value_digest: Digest = new_value.into();
        table.add_row(vec![
            updated_item_slot.to_string(),
            format!("Updated ({})", value_digest.to_hex()),
        ]);
    }

    println!("Storage changes:");
    println!("{table}");

    let mut table = create_dynamic_table(&["Asset Type", "Faucet ID", "Amount"]);

    for asset in account_delta.vault().added_assets.iter() {
        let (asset_type, faucet_id, amount) = match asset {
            Asset::Fungible(fungible_asset) => {
                ("Fungible Asset", fungible_asset.faucet_id(), fungible_asset.amount())
            },
            Asset::NonFungible(non_fungible_asset) => {
                ("Non Fungible Asset", non_fungible_asset.faucet_id(), 1)
            },
        };
        table.add_row(vec![asset_type, &faucet_id.to_hex(), &format!("+{}", amount)]);
    }

    for asset in account_delta.vault().removed_assets.iter() {
        let (asset_type, faucet_id, amount) = match asset {
            Asset::Fungible(fungible_asset) => {
                ("Fungible Asset", fungible_asset.faucet_id(), fungible_asset.amount())
            },
            Asset::NonFungible(non_fungible_asset) => {
                ("Non Fungible Asset", non_fungible_asset.faucet_id(), 1)
            },
        };
        table.add_row(vec![asset_type, &faucet_id.to_hex(), &format!("-{}", amount)]);
    }

    println!("Vault changes:");
    println!("{table}");

    if let Some(new_nonce) = account_delta.nonce() {
        println!("New nonce: {new_nonce}.")
    } else {
        println!("No nonce changes.")
    }
}
