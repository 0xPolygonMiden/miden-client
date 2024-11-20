use std::{io, sync::Arc};

use clap::{Parser, ValueEnum};
use miden_client::{
    accounts::AccountId,
    assets::{FungibleAsset, NonFungibleDeltaAction},
    crypto::{Digest, FeltRng},
    notes::{build_swap_tag, get_input_note_with_id_prefix, NoteType as MidenNoteType},
    transactions::{
        PaymentTransactionData, SwapTransactionData, TransactionRequest, TransactionResult,
    },
    Client,
};
use miden_tx_prover::RemoteTransactionProver;
use tracing::info;

use crate::{
    create_dynamic_table,
    utils::{
        get_input_acc_id_by_prefix_or_default, load_config_file, load_faucet_details_map,
        parse_account_id, SHARED_TOKEN_DOCUMENTATION,
    },
};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum NoteType {
    Public,
    Private,
}

impl From<&NoteType> for MidenNoteType {
    fn from(note_type: &NoteType) -> Self {
        match note_type {
            NoteType::Public => MidenNoteType::Public,
            NoteType::Private => MidenNoteType::Private,
        }
    }
}

#[derive(Debug, Parser, Clone)]
/// Mint tokens from a fungible faucet to a wallet.
pub struct MintCmd {
    /// Target account ID or its hex prefix
    #[clap(short = 't', long = "target")]
    target_account_id: String,

    /// Asset to be minted.
    #[clap(short, long, help=format!("Asset to be minted.\n{SHARED_TOKEN_DOCUMENTATION}"))]
    asset: String,

    #[clap(short, long, value_enum)]
    note_type: NoteType,
    /// Flag to submit the executed transaction without asking for confirmation
    #[clap(long, default_value_t = false)]
    force: bool,
}

impl MintCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), String> {
        let force = self.force;
        let faucet_details_map = load_faucet_details_map()?;

        let fungible_asset = faucet_details_map.parse_fungible_asset(&self.asset)?;

        let target_account_id = parse_account_id(&client, self.target_account_id.as_str()).await?;

        let transaction_request = TransactionRequest::mint_fungible_asset(
            fungible_asset,
            target_account_id,
            (&self.note_type).into(),
            client.rng(),
        )
        .map_err(|err| err.to_string())?;

        execute_transaction(&mut client, fungible_asset.faucet_id(), transaction_request, force)
            .await
    }
}

#[derive(Debug, Parser, Clone)]
/// Create a pay-to-id transaction.
pub struct SendCmd {
    /// Sender account ID or its hex prefix. If none is provided, the default account's ID is used
    /// instead
    #[clap(short = 's', long = "sender")]
    sender_account_id: Option<String>,
    /// Target account ID or its hex prefix
    #[clap(short = 't', long = "target")]
    target_account_id: String,

    /// Asset to be sent.
    #[clap(short, long, help=format!("Asset to be sent.\n{SHARED_TOKEN_DOCUMENTATION}"))]
    asset: String,

    #[clap(short, long, value_enum)]
    note_type: NoteType,
    /// Flag to submit the executed transaction without asking for confirmation
    #[clap(long, default_value_t = false)]
    force: bool,
    /// Set the recall height for the transaction. If the note was not consumed by this height, the
    /// sender may consume it back.
    ///
    /// Setting this flag turns the transaction from a PayToId to a PayToIdWithRecall.
    #[clap(short, long)]
    recall_height: Option<u32>,
}

impl SendCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), String> {
        let force = self.force;

        let faucet_details_map = load_faucet_details_map()?;

        let fungible_asset = faucet_details_map.parse_fungible_asset(&self.asset)?;

        // try to use either the provided argument or the default account
        let sender_account_id =
            get_input_acc_id_by_prefix_or_default(&client, self.sender_account_id.clone()).await?;
        let target_account_id = parse_account_id(&client, self.target_account_id.as_str()).await?;

        let payment_transaction = PaymentTransactionData::new(
            vec![fungible_asset.into()],
            sender_account_id,
            target_account_id,
        );

        let transaction_request = TransactionRequest::pay_to_id(
            payment_transaction,
            self.recall_height,
            (&self.note_type).into(),
            client.rng(),
        )
        .map_err(|err| err.to_string())?;

        execute_transaction(&mut client, sender_account_id, transaction_request, force).await
    }
}

#[derive(Debug, Parser, Clone)]
/// Create a swap transaction.
pub struct SwapCmd {
    /// Sender account ID or its hex prefix. If none is provided, the default account's ID is used
    /// instead
    #[clap(short = 's', long = "source")]
    sender_account_id: Option<String>,

    /// Asset offered
    #[clap(long = "offered-asset", help=format!("Asset offered.\n{SHARED_TOKEN_DOCUMENTATION}"))]
    offered_asset: String,

    /// Asset requested
    #[clap(short, long, help=format!("Asset requested.\n{SHARED_TOKEN_DOCUMENTATION}"))]
    requested_asset: String,

    #[clap(short, long, value_enum)]
    note_type: NoteType,
    /// Flag to submit the executed transaction without asking for confirmation
    #[clap(long, default_value_t = false)]
    force: bool,
}

impl SwapCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), String> {
        let force = self.force;

        let faucet_details_map = load_faucet_details_map()?;

        let offered_fungible_asset =
            faucet_details_map.parse_fungible_asset(&self.offered_asset)?;
        let requested_fungible_asset =
            faucet_details_map.parse_fungible_asset(&self.requested_asset)?;

        // try to use either the provided argument or the default account
        let sender_account_id =
            get_input_acc_id_by_prefix_or_default(&client, self.sender_account_id.clone()).await?;

        let swap_transaction = SwapTransactionData::new(
            sender_account_id,
            offered_fungible_asset.into(),
            requested_fungible_asset.into(),
        );

        let transaction_request = TransactionRequest::swap(
            swap_transaction.clone(),
            (&self.note_type).into(),
            client.rng(),
        )
        .map_err(|err| err.to_string())?;

        execute_transaction(&mut client, sender_account_id, transaction_request, force).await?;

        let payback_note_tag: u32 = build_swap_tag(
            (&self.note_type).into(),
            &swap_transaction.offered_asset(),
            &swap_transaction.requested_asset(),
        )
        .map_err(|err| err.to_string())?
        .into();
        println!(
            "To receive updates about the payback Swap Note run `miden tags add {}`",
            payback_note_tag
        );

        Ok(())
    }
}

#[derive(Debug, Parser, Clone)]
/// Consume with the account corresponding to `account_id` all of the notes from `list_of_notes`.
/// If no account ID is provided, the default one is used. If no notes are provided, any notes
/// that are identified to be owned by the account ID are consumed.
pub struct ConsumeNotesCmd {
    /// The account ID to be used to consume the note or its hex prefix. If none is provided, the
    /// default account's ID is used instead
    #[clap(short = 'a', long = "account")]
    account_id: Option<String>,
    /// A list of note IDs or the hex prefixes of their corresponding IDs
    list_of_notes: Vec<String>,
    /// Flag to submit the executed transaction without asking for confirmation
    #[clap(short, long, default_value_t = false)]
    force: bool,
}

impl ConsumeNotesCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), String> {
        let force = self.force;

        let mut list_of_notes = Vec::new();
        for note_id in &self.list_of_notes {
            let note_record = get_input_note_with_id_prefix(&client, note_id)
                .await
                .map_err(|err| err.to_string())?;
            list_of_notes.push(note_record.id());
        }

        let account_id =
            get_input_acc_id_by_prefix_or_default(&client, self.account_id.clone()).await?;

        if list_of_notes.is_empty() {
            info!("No input note IDs provided, getting all notes consumable by {}", account_id);
            let consumable_notes = client.get_consumable_notes(Some(account_id)).await?;

            list_of_notes.extend(consumable_notes.iter().map(|(note, _)| note.id()));
        }

        if list_of_notes.is_empty() {
            return Err(format!("No input notes were provided and the store does not contain any notes consumable by {account_id}"));
        }

        let transaction_request = TransactionRequest::consume_notes(list_of_notes);

        execute_transaction(&mut client, account_id, transaction_request, force).await
    }
}

// EXECUTE TRANSACTION
// ================================================================================================

async fn execute_transaction(
    client: &mut Client<impl FeltRng>,
    account_id: AccountId,
    transaction_request: TransactionRequest,
    force: bool,
) -> Result<(), String> {
    println!("Executing transaction...");
    let transaction_execution_result =
        client.new_transaction(account_id, transaction_request).await?;

    // Show delta and ask for confirmation
    print_transaction_details(&transaction_execution_result)?;
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

    let (cli_config, _) = load_config_file()?;
    match &cli_config.remote_prover_endpoint {
        Some(proving_url) => {
            let remote_prover = Arc::new(RemoteTransactionProver::new(&proving_url.to_string()));
            client
                .submit_transaction_with_prover(transaction_execution_result, remote_prover)
                .await?;
        },
        None => client.submit_transaction(transaction_execution_result).await?,
    };

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

fn print_transaction_details(transaction_result: &TransactionResult) -> Result<(), String> {
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

    let has_storage_changes = !account_delta.storage().is_empty();
    if has_storage_changes {
        let mut table = create_dynamic_table(&["Storage Slot", "Effect"]);

        for (updated_item_slot, new_value) in account_delta.storage().values() {
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

    if !account_delta.vault().is_empty() {
        let faucet_details_map = load_faucet_details_map()?;
        let mut table = create_dynamic_table(&["Asset Type", "Faucet ID", "Amount"]);

        for (faucet_id, amount) in account_delta.vault().fungible().iter() {
            let asset = FungibleAsset::new(*faucet_id, amount.unsigned_abs())
                .map_err(|err| err.to_string())?;
            let (faucet_fmt, amount_fmt) = faucet_details_map.format_fungible_asset(&asset)?;

            if amount.is_positive() {
                table.add_row(vec!["Fungible Asset", &faucet_fmt, &format!("+{}", amount_fmt)]);
            } else {
                table.add_row(vec!["Fungible Asset", &faucet_fmt, &format!("-{}", amount_fmt)]);
            }
        }

        for (asset, action) in account_delta.vault().non_fungible().iter() {
            match action {
                NonFungibleDeltaAction::Add => {
                    table.add_row(vec!["Non Fungible Asset", &asset.faucet_id().to_hex(), "1"]);
                },
                NonFungibleDeltaAction::Remove => {
                    table.add_row(vec!["Non Fungible Asset", &asset.faucet_id().to_hex(), "-1"]);
                },
            }
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

    Ok(())
}
