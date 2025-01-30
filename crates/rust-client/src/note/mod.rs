//! Contains the Client APIs related to notes. Notes can contain assets and scripts that are
//! executed as part of transactions.
//!
//! This module enables the tracking, retrieval, and processing of notes.
//! It offers methods to query input and output notes from the store, check their consumability,
//! compile note scripts, and retrieve notes based on partial ID matching.
//!
//! ## Overview
//!
//! The module exposes APIs to:
//!
//! - Retrieve input notes and output notes.
//! - Determine the consumability of notes using the [NoteScreener].
//! - Compile note scripts from source code with `compile_note_script`.
//! - Retrieve an input note by a prefix of its ID using the helper function
//!   [get_input_note_with_id_prefix].
//!
//! ## Example
//!
//! ```rust
//! use miden_client::{
//!     Client,
//!     note::{get_input_note_with_id_prefix, NoteScreener},
//!     store::NoteFilter,
//!     crypto::FeltRng,
//! };
//! use miden_objects::account::AccountId;
//!
//! # async fn example(client: &Client<impl FeltRng>) -> Result<(), Box<dyn std::error::Error>> {
//! // Retrieve all committed input notes
//! let input_notes = client.get_input_notes(NoteFilter::Committed).await?;
//! println!("Found {} committed input notes.", input_notes.len());
//!
//! // Check consumability for a specific note
//! if let Some(note) = input_notes.first() {
//!     let consumability = client.get_note_consumability(note.clone()).await?;
//!     println!("Note consumability: {:?}", consumability);
//! }
//!
//! // Retrieve an input note by a partial ID match
//! let note_prefix = "0x70b7ec";
//! match get_input_note_with_id_prefix(client, note_prefix).await {
//!     Ok(note) => println!("Found note with matching prefix: {}", note.id().to_hex()),
//!     Err(err) => println!("Error retrieving note: {:?}", err),
//! }
//!
//! // Compile the note script
//! let script_src = "begin push.9 push.12 add end";
//! let note_script = client.compile_note_script(script_src)?;
//! println!("Compiled note script successfully.");
//!
//! # Ok(())
//! # }
//! ```
//!
//! For more details on the API and error handling, see the documentation for the specific functions
//! and types in this module.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::ToString,
    vec::Vec,
};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{account::AccountId, crypto::rand::FeltRng};

use crate::{
    store::{InputNoteRecord, NoteFilter, OutputNoteRecord},
    Client, ClientError, IdPrefixFetchError,
};

pub mod script_roots;

mod import;
mod note_screener;

// RE-EXPORTS
// ================================================================================================

pub use miden_lib::note::{
    create_p2id_note, create_p2idr_note, create_swap_note,
    utils::{build_p2id_recipient, build_swap_tag},
};
pub use miden_objects::{
    block::BlockNumber,
    note::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteFile, NoteId,
        NoteInclusionProof, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType,
        Nullifier,
    },
    NoteError,
};
pub use note_screener::{NoteConsumability, NoteRelevance, NoteScreener, NoteScreenerError};

/// Contains functions to simplify standard note scripts creation.
pub mod scripts {
    pub use miden_lib::note::scripts::{p2id, p2idr, swap};
}

/// Note retrieval methods.
impl<R: FeltRng> Client<R> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes managed by the client from the store.
    ///
    /// # Errors
    ///
    /// Returns a [ClientError::StoreError] if the filter is [NoteFilter::Unique] and there is no
    /// Note with the provided ID.
    pub async fn get_input_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, ClientError> {
        self.store.get_input_notes(filter).await.map_err(|err| err.into())
    }

    /// Returns the input notes and their consumability.
    ///
    /// If account_id is None then all consumable input notes are returned.
    pub async fn get_consumable_notes(
        &self,
        account_id: Option<AccountId>,
    ) -> Result<Vec<(InputNoteRecord, Vec<NoteConsumability>)>, ClientError> {
        let commited_notes = self.store.get_input_notes(NoteFilter::Committed).await?;

        let note_screener = NoteScreener::new(self.store.clone());

        let mut relevant_notes = Vec::new();
        for input_note in commited_notes {
            let mut account_relevance =
                note_screener.check_relevance(&input_note.clone().try_into()?).await?;

            if let Some(account_id) = account_id {
                account_relevance.retain(|(id, _)| *id == account_id);
            }

            if account_relevance.is_empty() {
                continue;
            }

            relevant_notes.push((input_note, account_relevance));
        }

        Ok(relevant_notes)
    }

    /// Returns the consumability of the provided note.
    pub async fn get_note_consumability(
        &self,
        note: InputNoteRecord,
    ) -> Result<Vec<NoteConsumability>, ClientError> {
        let note_screener = NoteScreener::new(self.store.clone());
        note_screener
            .check_relevance(&note.clone().try_into()?)
            .await
            .map_err(|err| err.into())
    }

    /// Retrieves the input note given a [NoteId]. Returns `None` if the note is not found.
    pub async fn get_input_note(
        &self,
        note_id: NoteId,
    ) -> Result<Option<InputNoteRecord>, ClientError> {
        Ok(self.store.get_input_notes(NoteFilter::Unique(note_id)).await?.pop())
    }

    // OUTPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns output notes managed by this client.
    pub async fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, ClientError> {
        self.store.get_output_notes(filter).await.map_err(|err| err.into())
    }

    /// Retrieves the output note given a [NoteId]. Returns `None` if the note is not found.
    pub async fn get_output_note(
        &self,
        note_id: NoteId,
    ) -> Result<Option<OutputNoteRecord>, ClientError> {
        Ok(self.store.get_output_notes(NoteFilter::Unique(note_id)).await?.pop())
    }

    /// Compiles the provided program into a [NoteScript].
    ///
    /// The assembler uses the debug mode if the client was instantiated with debug mode on.
    pub fn compile_note_script(&self, note_script: &str) -> Result<NoteScript, ClientError> {
        let assembler = TransactionKernel::assembler().with_debug_mode(self.in_debug_mode);
        NoteScript::compile(note_script, assembler).map_err(ClientError::NoteError)
    }
}

/// Returns the client input note whose ID starts with `note_id_prefix`.
///
/// # Errors
///
/// - Returns [IdPrefixFetchError::NoMatch] if we were unable to find any note where
///   `note_id_prefix` is a prefix of its ID.
/// - Returns [IdPrefixFetchError::MultipleMatches] if there were more than one note found where
///   `note_id_prefix` is a prefix of its ID.
pub async fn get_input_note_with_id_prefix<R: FeltRng>(
    client: &Client<R>,
    note_id_prefix: &str,
) -> Result<InputNoteRecord, IdPrefixFetchError> {
    let mut input_note_records = client
        .get_input_notes(NoteFilter::All)
        .await
        .map_err(|err| {
            tracing::error!("Error when fetching all notes from the store: {err}");
            IdPrefixFetchError::NoMatch(format!("note ID prefix {note_id_prefix}").to_string())
        })?
        .into_iter()
        .filter(|note_record| note_record.id().to_hex().starts_with(note_id_prefix))
        .collect::<Vec<_>>();

    if input_note_records.is_empty() {
        return Err(IdPrefixFetchError::NoMatch(
            format!("note ID prefix {note_id_prefix}").to_string(),
        ));
    }
    if input_note_records.len() > 1 {
        let input_note_record_ids = input_note_records
            .iter()
            .map(|input_note_record| input_note_record.id())
            .collect::<Vec<_>>();
        tracing::error!(
            "Multiple notes found for the prefix {}: {:?}",
            note_id_prefix,
            input_note_record_ids
        );
        return Err(IdPrefixFetchError::MultipleMatches(
            format!("note ID prefix {note_id_prefix}").to_string(),
        ));
    }

    Ok(input_note_records
        .pop()
        .expect("input_note_records should always have one element"))
}

// NOTE UPDATES
// ------------------------------------------------------------------------------------------------

/// Contains note changes to apply to the store.
#[derive(Clone, Debug, Default)]
pub struct NoteUpdates {
    /// A map of new and updated input note records to be upserted in the store.
    updated_input_notes: BTreeMap<NoteId, InputNoteRecord>,
    /// A map of updated output note records to be upserted in the store.
    updated_output_notes: BTreeMap<NoteId, OutputNoteRecord>,
}

impl NoteUpdates {
    /// Creates a [NoteUpdates].
    pub fn new(
        updated_input_notes: Vec<InputNoteRecord>,
        updated_output_notes: Vec<OutputNoteRecord>,
    ) -> Self {
        Self {
            updated_input_notes: updated_input_notes
                .into_iter()
                .map(|note| (note.id(), note))
                .collect(),
            updated_output_notes: updated_output_notes
                .into_iter()
                .map(|note| (note.id(), note))
                .collect(),
        }
    }

    /// Returns all input note records that have been updated.
    pub fn updated_input_notes(&self) -> impl Iterator<Item = &InputNoteRecord> {
        self.updated_input_notes.values()
    }

    /// Returns all updated output note records that have been updated.
    pub fn updated_output_notes(&self) -> impl Iterator<Item = &OutputNoteRecord> {
        self.updated_output_notes.values()
    }

    /// Returns whether no new note-related information has been retrieved.
    pub fn is_empty(&self) -> bool {
        self.updated_input_notes.is_empty() && self.updated_output_notes.is_empty()
    }

    /// Returns any note that has been committed into the chain in this update (either new or
    /// already locally tracked)
    pub fn committed_input_notes(&self) -> impl Iterator<Item = &InputNoteRecord> {
        self.updated_input_notes.values().filter(|note| note.is_committed())
    }

    /// Returns the IDs of all notes that have been committed (previously locally tracked).
    pub fn committed_note_ids(&self) -> BTreeSet<NoteId> {
        let committed_output_note_ids = self
            .updated_output_notes
            .values()
            .filter_map(|note_record| note_record.is_committed().then_some(note_record.id()));

        let committed_input_note_ids = self
            .updated_input_notes
            .values()
            .filter_map(|note_record| note_record.is_committed().then_some(note_record.id()));

        BTreeSet::from_iter(committed_input_note_ids.chain(committed_output_note_ids))
    }

    /// Returns the IDs of all notes that have been consumed.
    pub fn consumed_note_ids(&self) -> BTreeSet<NoteId> {
        let consumed_output_note_ids = self
            .updated_output_notes
            .values()
            .filter_map(|note_record| note_record.is_consumed().then_some(note_record.id()));

        let consumed_input_note_ids = self
            .updated_input_notes
            .values()
            .filter_map(|note_record| note_record.is_consumed().then_some(note_record.id()));

        BTreeSet::from_iter(consumed_input_note_ids.chain(consumed_output_note_ids))
    }

    pub fn insert_or_ignore_notes(
        &mut self,
        input_notes: &Vec<InputNoteRecord>,
        output_notes: &Vec<OutputNoteRecord>,
    ) {
        for note in input_notes {
            self.updated_input_notes.entry(note.id()).or_insert(note.clone());
        }

        for note in output_notes {
            self.updated_output_notes.entry(note.id()).or_insert(note.clone());
        }
    }

    /// Returns a mutable reference to the input note record with the provided ID if it exists.
    pub fn get_input_note_by_id(&mut self, note_id: &NoteId) -> Option<&mut InputNoteRecord> {
        self.updated_input_notes.get_mut(note_id)
    }

    /// Returns a mutable reference to the output note record with the provided ID if it exists.
    pub fn get_output_note_by_id(&mut self, note_id: &NoteId) -> Option<&mut OutputNoteRecord> {
        self.updated_output_notes.get_mut(note_id)
    }

    /// Returns a mutable reference to the input note record with the provided nullifier if it
    /// exists.
    pub fn get_input_note_by_nullifier(
        &mut self,
        nullifier: Nullifier,
    ) -> Option<&mut InputNoteRecord> {
        self.updated_input_notes.values_mut().find(|note| note.nullifier() == nullifier)
    }

    /// Returns a mutable reference to the output note record with the provided nullifier if it
    /// exists.
    pub fn get_output_note_by_nullifier(
        &mut self,
        nullifier: Nullifier,
    ) -> Option<&mut OutputNoteRecord> {
        self.updated_output_notes
            .values_mut()
            .find(|note| note.nullifier() == Some(nullifier))
    }
}
