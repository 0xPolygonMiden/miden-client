use clap::ValueEnum;
use comfy_table::{presets, Attribute, Cell, ContentArrangement, Table};
use miden_client::{
    account::AccountId,
    asset::Asset,
    crypto::{Digest, FeltRng},
    note::{
        get_input_note_with_id_prefix,
        script_roots::{P2ID, P2IDR, SWAP},
        NoteConsumability, NoteInputs, NoteMetadata,
    },
    store::{InputNoteRecord, NoteFilter as ClientNoteFilter, OutputNoteRecord},
    Client, ClientError, IdPrefixFetchError,
};

use crate::{
    create_dynamic_table, errors::CliError, get_output_note_with_id_prefix,
    utils::load_faucet_details_map, Parser,
};

#[derive(Clone, Debug, ValueEnum)]
pub enum NoteFilter {
    All,
    Expected,
    Committed,
    Consumed,
    Processing,
    Consumable,
}

impl TryInto<ClientNoteFilter> for NoteFilter {
    type Error = String;

    fn try_into(self) -> Result<ClientNoteFilter, Self::Error> {
        match self {
            NoteFilter::All => Ok(ClientNoteFilter::All),
            NoteFilter::Expected => Ok(ClientNoteFilter::Expected),
            NoteFilter::Committed => Ok(ClientNoteFilter::Committed),
            NoteFilter::Consumed => Ok(ClientNoteFilter::Consumed),
            NoteFilter::Processing => Ok(ClientNoteFilter::Processing),
            NoteFilter::Consumable => Err("Consumable filter is not supported".to_string()),
        }
    }
}

#[derive(Debug, Parser, Clone)]
#[clap(about = "View and manage notes")]
pub struct NotesCmd {
    /// List notes with the specified filter. If no filter is provided, all notes will be listed.
    #[clap(short, long, group = "action", default_missing_value="all", num_args=0..=1, value_name = "filter")]
    list: Option<NoteFilter>,
    /// Show note with the specified ID.
    #[clap(short, long, group = "action", value_name = "note_id")]
    show: Option<String>,
    /// (only has effect on `--list consumable`) Account ID used to filter list. Only notes
    /// consumable by this account will be shown.
    #[clap(short, long, value_name = "account_id")]
    account_id: Option<String>,
}

impl NotesCmd {
    pub async fn execute(&self, client: Client<impl FeltRng>) -> Result<(), CliError> {
        match self {
            NotesCmd { list: Some(NoteFilter::Consumable), .. } => {
                list_consumable_notes(client, &None).await?;
            },
            NotesCmd { list: Some(filter), .. } => {
                list_notes(
                    client,
                    filter.clone().try_into().expect("Filter shouldn't be consumable"),
                )
                .await?;
            },
            NotesCmd { show: Some(id), .. } => {
                show_note(client, id.to_owned()).await?;
            },
            _ => {
                list_notes(client, ClientNoteFilter::All).await?;
            },
        }
        Ok(())
    }
}

struct CliNoteSummary {
    id: String,
    script_hash: String,
    assets_hash: String,
    inputs_commitment: String,
    serial_num: String,
    note_type: String,
    state: String,
    tag: String,
    sender: String,
    exportable: bool,
}

// LIST NOTES
// ================================================================================================
async fn list_notes(
    client: Client<impl FeltRng>,
    filter: ClientNoteFilter,
) -> Result<(), CliError> {
    let input_notes = client
        .get_input_notes(filter.clone())
        .await?
        .into_iter()
        .map(|input_note_record| note_summary(Some(&input_note_record), None))
        .collect::<Result<Vec<CliNoteSummary>, CliError>>()?;
    let output_notes = client
        .get_output_notes(filter.clone())
        .await?
        .into_iter()
        .map(|output_note_record| note_summary(None, Some(&output_note_record)))
        .collect::<Result<Vec<CliNoteSummary>, CliError>>()?;

    print_notes_summary(input_notes, "Input Notes")?;
    print_notes_summary(output_notes, "Output Notes")
}

// SHOW NOTE
// ================================================================================================
async fn show_note(client: Client<impl FeltRng>, note_id: String) -> Result<(), CliError> {
    let input_note_record = get_input_note_with_id_prefix(&client, &note_id).await;
    let output_note_record = get_output_note_with_id_prefix(&client, &note_id).await;

    // If we don't find an input note nor an output note return an error
    if matches!(input_note_record, Err(IdPrefixFetchError::NoMatch(_)))
        && matches!(output_note_record, Err(IdPrefixFetchError::NoMatch(_)))
    {
        return Err(CliError::Import(
            "The specified note ID hex prefix did not match any note".to_string(),
        ));
    }

    // If either one of the two match with multiple notes return an error
    if matches!(input_note_record, Err(IdPrefixFetchError::MultipleMatches(_)))
        || matches!(output_note_record, Err(IdPrefixFetchError::MultipleMatches(_)))
    {
        return Err(CliError::Import(
            "The specified note ID hex prefix matched with more than one note.".to_string(),
        ));
    }

    let input_note_record = input_note_record.ok();
    let output_note_record = output_note_record.ok();

    // If we match one note as the input note and another one as the output note return an error
    match (&input_note_record, &output_note_record) {
        (Some(input_record), Some(output_record)) if input_record.id() != output_record.id() => {
            return Err(CliError::Import(
                "The specified note ID hex prefix matched with more than one note.".to_string(),
            ));
        },
        _ => {},
    }

    let mut table = create_dynamic_table(&["Note Information"]);
    table
        .load_preset(presets::UTF8_HORIZONTAL_ONLY)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);

    let CliNoteSummary {
        id,
        mut script_hash,
        assets_hash,
        inputs_commitment,
        serial_num,
        note_type,
        state,
        tag,
        sender,
        exportable,
    } = note_summary(input_note_record.as_ref(), output_note_record.as_ref())?;
    table.add_row(vec![Cell::new("ID"), Cell::new(id)]);
    match script_hash.clone().as_str() {
        P2ID => script_hash += " (P2ID)",
        P2IDR => script_hash += " (P2IDR)",
        SWAP => script_hash += " (SWAP)",
        _ => {},
    };

    table.add_row(vec![Cell::new("Script Hash"), Cell::new(script_hash)]);
    table.add_row(vec![Cell::new("Assets Hash"), Cell::new(assets_hash)]);
    table.add_row(vec![Cell::new("Inputs Hash"), Cell::new(inputs_commitment)]);
    table.add_row(vec![Cell::new("Serial Number"), Cell::new(serial_num)]);
    table.add_row(vec![Cell::new("Type"), Cell::new(note_type)]);
    table.add_row(vec![Cell::new("State"), Cell::new(state)]);
    table.add_row(vec![Cell::new("Tag"), Cell::new(tag)]);
    table.add_row(vec![Cell::new("Sender"), Cell::new(sender)]);
    table.add_row(vec![Cell::new("Exportable"), Cell::new(if exportable { "✔" } else { "✘" })]);

    println!("{table}");

    let inputs = match (&input_note_record, &output_note_record) {
        (Some(record), _) => {
            let details = record.details();
            Some(details.inputs().values().to_vec())
        },
        (_, Some(record)) => {
            record.recipient().map(|recipient| recipient.inputs().values().to_vec())
        },
        (None, None) => {
            panic!("One of the two records should be Some")
        },
    };

    let assets = input_note_record
        .map(|record| record.assets().clone())
        .or(output_note_record.map(|record| record.assets().clone()))
        .expect("One of the two records should be Some");

    // print note vault
    let mut table = create_dynamic_table(&["Note Assets"]);
    table
        .load_preset(presets::UTF8_HORIZONTAL_ONLY)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);

    table.add_row(vec![
        Cell::new("Type").add_attribute(Attribute::Bold),
        Cell::new("Faucet ID").add_attribute(Attribute::Bold),
        Cell::new("Amount").add_attribute(Attribute::Bold),
    ]);
    let faucet_details_map = load_faucet_details_map()?;
    let assets = assets.iter();

    for asset in assets {
        let (asset_type, faucet, amount) = match asset {
            Asset::Fungible(fungible_asset) => {
                let (faucet, amount) = faucet_details_map.format_fungible_asset(fungible_asset)?;
                ("Fungible Asset", faucet, amount)
            },
            Asset::NonFungible(non_fungible_asset) => (
                "Non Fungible Asset",
                non_fungible_asset.faucet_id_prefix().to_hex(),
                1.0.to_string(),
            ),
        };
        table.add_row(vec![asset_type, &faucet, &amount.to_string()]);
    }
    println!("{table}");

    if inputs.is_some() {
        let inputs = inputs.expect("Inputs should be Some");
        let inputs = NoteInputs::new(inputs.clone()).map_err(ClientError::NoteError)?;
        let mut table = create_dynamic_table(&["Note Inputs"]);
        table
            .load_preset(presets::UTF8_HORIZONTAL_ONLY)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth);
        table.add_row(vec![
            Cell::new("Index").add_attribute(Attribute::Bold),
            Cell::new("Value").add_attribute(Attribute::Bold),
        ]);

        inputs.values().iter().enumerate().for_each(|(idx, input)| {
            table.add_row(vec![Cell::new(idx).add_attribute(Attribute::Bold), Cell::new(input)]);
        });
        println!("{table}");
    };

    Ok(())
}

// LIST CONSUMABLE INPUT NOTES
// ================================================================================================
async fn list_consumable_notes(
    client: Client<impl FeltRng>,
    account_id: &Option<String>,
) -> Result<(), CliError> {
    let account_id = match account_id {
        Some(id) => Some(
            AccountId::from_hex(id.as_str())
                .map_err(|err| CliError::AccountId(err, "Invalid account ID".to_string()))?,
        ),
        None => None,
    };
    let notes = client.get_consumable_notes(account_id).await?;
    print_consumable_notes_summary(&notes)?;
    Ok(())
}

// HELPERS
// ================================================================================================
fn print_notes_summary<I>(notes: I, header: &str) -> Result<(), CliError>
where
    I: IntoIterator<Item = CliNoteSummary>,
{
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_NO_BORDERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth);
    table.set_header(vec![Cell::new(header).add_attribute(Attribute::Bold)]);
    println!("\n{table}");

    for summary in notes {
        println!(" {} {}", summary.id, summary.state);
    }

    Ok(())
}

fn print_consumable_notes_summary<'a, I>(notes: I) -> Result<(), CliError>
where
    I: IntoIterator<Item = &'a (InputNoteRecord, Vec<NoteConsumability>)>,
{
    let mut table = create_dynamic_table(&["Note ID", "Account ID", "Relevance"]);

    for (note, relevances) in notes {
        for relevance in relevances {
            table.add_row(vec![
                note.id().to_hex(),
                relevance.0.to_string(),
                relevance.1.to_string(),
            ]);
        }
    }

    println!("{table}");

    Ok(())
}

fn note_record_type(note_record_metadata: Option<&NoteMetadata>) -> String {
    match note_record_metadata {
        Some(metadata) => match metadata.note_type() {
            miden_client::note::NoteType::Private => "Private",
            miden_client::note::NoteType::Encrypted => "Encrypted",
            miden_client::note::NoteType::Public => "Public",
        },
        None => "-",
    }
    .to_string()
}

/// Given that one of the two records is Some, this function will return a summary of the note.
fn note_summary(
    input_note_record: Option<&InputNoteRecord>,
    output_note_record: Option<&OutputNoteRecord>,
) -> Result<CliNoteSummary, CliError> {
    let note_id = input_note_record
        .map(|record| record.id())
        .or(output_note_record.map(|record| record.id()))
        .expect("One of the two records should be Some");

    let assets_hash_str = input_note_record
        .map(|record| record.assets().commitment().to_string())
        .or(output_note_record.map(|record| record.assets().commitment().to_string()))
        .expect("One of the two records should be Some");

    let (inputs_commitment_str, serial_num, script_hash_str) =
        match (input_note_record, output_note_record) {
            (Some(record), _) => {
                let details = record.details();
                (
                    details.inputs().commitment().to_string(),
                    Digest::new(details.serial_num()).to_string(),
                    details.script().hash().to_string(),
                )
            },
            (None, Some(record)) if record.recipient().is_some() => {
                let recipient = record.recipient().expect("output record should have recipient");
                (
                    recipient.inputs().commitment().to_string(),
                    Digest::new(recipient.serial_num()).to_string(),
                    recipient.script().hash().to_string(),
                )
            },
            (None, Some(_record)) => ("-".to_string(), "-".to_string(), "-".to_string()),
            (None, None) => panic!("One of the two records should be Some"),
        };

    let note_type = note_record_type(
        input_note_record
            .and_then(|record| record.metadata())
            .or(output_note_record.map(|record| record.metadata())),
    );

    let state = input_note_record
        .map(|record| record.state().to_string())
        .or(output_note_record.map(|record| record.state().to_string()))
        .expect("One of the two records should be Some");

    let note_metadata = input_note_record
        .map(|record| record.metadata())
        .or(output_note_record.map(|record| Some(record.metadata())))
        .expect("One of the two records should be Some");

    let note_tag_str = note_metadata
        .map(|metadata| metadata.tag().to_string())
        .unwrap_or("-".to_string());

    let note_sender_str = note_metadata
        .map(|metadata| metadata.sender().to_string())
        .unwrap_or("-".to_string());

    Ok(CliNoteSummary {
        id: note_id.inner().to_string(),
        script_hash: script_hash_str,
        assets_hash: assets_hash_str,
        inputs_commitment: inputs_commitment_str,
        serial_num,
        note_type,
        state,
        tag: note_tag_str,
        sender: note_sender_str,
        exportable: output_note_record.is_some(),
    })
}
