use std::{io, time::Instant};

use clap::{Parser, ValueEnum};
use miden_client::{
    rpc::NodeRpcClient,
    store::Store,
    transactions::{
        transaction_request::{PaymentTransactionData, SwapTransactionData, TransactionTemplate},
        TransactionResult,
    },
    Client,
};
use miden_objects::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    crypto::rand::FeltRng,
    notes::{NoteId, NoteType as MidenNoteType},
    Digest,
};
use miden_tx::auth::TransactionAuthenticator;
use tracing::info;

use super::{
    get_input_note_with_id_prefix,
    utils::{get_input_acc_id_by_prefix_or_default, parse_account_id, parse_fungible_asset},
};
use crate::cli::{create_dynamic_table, utils::build_swap_tag};

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

    /// Asset in the format `<AMOUNT>::<FAUCET_ID_HEX>`
    #[clap(short, long, value_parser = parse_fungible_asset)]
    asset: (u64, AccountId),

    #[clap(short, long, value_enum)]
    note_type: NoteType,
    /// Flag to submit the executed transaction without asking for confirmation
    #[clap(short, long, default_value_t = false)]
    force: bool,
}

impl MintCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        let force = self.force;
        let transaction_template = self.get_template(&client)?;
        execute_transaction(&mut client, transaction_template, force).await
    }

    fn get_template<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &Client<N, R, S, A>,
    ) -> Result<TransactionTemplate, String> {
        let faucet_id = self.asset.1;
        let fungible_asset =
            FungibleAsset::new(faucet_id, self.asset.0).map_err(|err| err.to_string())?;
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

    /// Asset in the format `<AMOUNT>::<FAUCET_ID_HEX>`
    #[clap(short, long, value_parser = parse_fungible_asset)]
    asset: (u64, AccountId),

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
}

impl SendCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        let force = self.force;
        let transaction_template = self.get_template(&client)?;
        execute_transaction(&mut client, transaction_template, force).await
    }

    fn get_template<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &Client<N, R, S, A>,
    ) -> Result<TransactionTemplate, String> {
        let faucet_id = self.asset.1;
        let fungible_asset = FungibleAsset::new(faucet_id, self.asset.0)
            .map_err(|err| err.to_string())?
            .into();

        // try to use either the provided argument or the default account
        let sender_account_id =
            get_input_acc_id_by_prefix_or_default(client, self.sender_account_id.clone())?;
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
/// Create a swap transaction.
pub struct SwapCmd {
    /// Sender account ID or its hex prefix. If none is provided, the default account's ID is used instead
    #[clap(short = 's', long = "source")]
    sender_account_id: Option<String>,

    /// offered Asset in the format `<AMOUNT>::<FAUCET_ID_HEX>`
    #[clap(long = "offered-asset", value_parser = parse_fungible_asset)]
    offered_asset: (u64, AccountId),

    /// requested Asset in the format `<AMOUNT>::<FAUCET_ID_HEX>`
    #[clap(short, long, value_parser = parse_fungible_asset)]
    requested_asset: (u64, AccountId),

    #[clap(short, long, value_enum)]
    note_type: NoteType,
    /// Flag to submit the executed transaction without asking for confirmation
    #[clap(long, default_value_t = false)]
    force: bool,
}

impl SwapCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        let force = self.force;
        let transaction_template = self.get_template(&client)?;
        execute_transaction(&mut client, transaction_template, force).await
    }

    fn get_template<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &Client<N, R, S, A>,
    ) -> Result<TransactionTemplate, String> {
        let offered_asset_faucet_id = self.offered_asset.1;
        let offered_fungible_asset =
            FungibleAsset::new(offered_asset_faucet_id, self.offered_asset.0)
                .map_err(|err| err.to_string())?
                .into();

        let requested_asset_faucet_id = self.requested_asset.1;
        let requested_fungible_asset =
            FungibleAsset::new(requested_asset_faucet_id, self.requested_asset.0)
                .map_err(|err| err.to_string())?
                .into();

        // try to use either the provided argument or the default account
        let sender_account_id =
            get_input_acc_id_by_prefix_or_default(client, self.sender_account_id.clone())?;

        let swap_transaction = SwapTransactionData::new(
            sender_account_id,
            offered_fungible_asset,
            requested_fungible_asset,
        );

        Ok(TransactionTemplate::Swap(swap_transaction, (&self.note_type).into()))
    }
}

#[derive(Debug, Parser, Clone)]
/// Consume with the account corresponding to `account_id` all of the notes from `list_of_notes`.
/// If no account ID is provided, the default one is used. If no notes are provided, any notes
/// that are identified to be owned by the account ID are consumed.
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
        &self,
        mut client: Client<N, R, S, A>,
    ) -> Result<(), String> {
        let force = self.force;
        let transaction_template = self.get_template(&client)?;
        execute_transaction(&mut client, transaction_template, force).await
    }

    fn get_template<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &Client<N, R, S, A>,
    ) -> Result<TransactionTemplate, String> {
        let mut list_of_notes = self
            .list_of_notes
            .iter()
            .map(|note_id| {
                get_input_note_with_id_prefix(client, note_id)
                    .map(|note_record| note_record.id())
                    .map_err(|err| err.to_string())
            })
            .collect::<Result<Vec<NoteId>, _>>()?;

        let account_id = get_input_acc_id_by_prefix_or_default(client, self.account_id.clone())?;

        if list_of_notes.is_empty() {
            info!("No input note IDs provided, getting all notes consumable by {}", account_id);
            let consumable_notes = client.get_consumable_notes(Some(account_id))?;

            list_of_notes.extend(consumable_notes.iter().map(|n| n.note.id()));
        }

        if list_of_notes.is_empty() {
            return Err(format!("No input notes were provided and the store does not contain any notes consumable by {account_id}"));
        }

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
    let transaction_request = client.build_transaction_request(transaction_template.clone())?;

    println!("Executing transaction...");
    let transaction_execution_result = client.new_transaction(transaction_request)?;

    // Show delta and ask for confirmation
    print_transaction_details(&transaction_execution_result);
    if !force {
        println!("\nContinue with proving and submission? Changes will be irreversible once the proof is finalized on the rollup (Y/N)");
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
    println!("Proving transaction...");
    let start = Instant::now();
    let proven_transaction =
        client.prove_transaction(transaction_execution_result.executed_transaction().clone())?;
    println!("Proving took: {}ms", start.elapsed().as_millis());
    println!("Submitting transaction to node and storing in database...");
    let start = Instant::now();
    client
        .submit_transaction(transaction_execution_result, proven_transaction)
        .await?;
    println!("Submission and storage took: {}ms", start.elapsed().as_millis());

    if let TransactionTemplate::Swap(swap_data, note_type) = transaction_template {
        let payback_note_tag: u32 = build_swap_tag(
            note_type,
            swap_data.offered_asset().faucet_id(),
            swap_data.requested_asset().faucet_id(),
        )
        .map_err(|err| err.to_string())?
        .into();
        println!(
            "To receive updates about the payback Swap Note run `miden tags add {}`",
            payback_note_tag
        );
    }

    println!("Succesfully created transaction.");
    println!("Transaction ID: {}", transaction_id);

    if output_notes.is_empty() {
        println!("The transaction did not generate any output notes.");
    } else {
        println!("Output notes:");
        output_notes.iter().for_each(|note_id| println!("\t- {}", note_id));
    }

    Ok(())
}

fn print_transaction_details(transaction_result: &TransactionResult) {
    println!("The transaction will have the following effects:\n");

    // INPUT NOTES
    let input_note_ids = transaction_result
        .executed_transaction()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();
    if input_note_ids.is_empty() {
        println!("No notes will be consumed.");
    } else {
        println!("The following notes will be consumed:");
        for input_note_id in input_note_ids {
            println!("\t- {}", input_note_id.to_hex());
        }
    }
    println!();

    // OUTPUT NOTES
    let output_note_count = transaction_result.executed_transaction().output_notes().iter().count();
    if output_note_count == 0 {
        println!("No notes will be created as a result of this transaction.");
    } else {
        println!("{output_note_count} notes will be created as a result of this transaction.");
    }
    println!();

    // ACCOUNT CHANGES
    println!(
        "The account with ID {} will be modified as follows:",
        transaction_result.executed_transaction().account_id()
    );

    let account_delta = transaction_result.account_delta();

    let has_storage_changes = !account_delta.storage().cleared_items.is_empty()
        || !account_delta.storage().updated_items.is_empty();
    if has_storage_changes {
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
    } else {
        println!("Account Storage will not be changed.");
    }

    let has_vault_changes = !account_delta.vault().added_assets.is_empty()
        || !account_delta.vault().removed_assets.is_empty();

    if has_vault_changes {
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
    } else {
        println!("Account Vault will not be changed.");
    }

    if let Some(new_nonce) = account_delta.nonce() {
        println!("New nonce: {new_nonce}.")
    } else {
        println!("No nonce changes.")
    }
}
