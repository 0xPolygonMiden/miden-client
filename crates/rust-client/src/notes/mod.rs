//! Contains the Client APIs related to notes. Notes can contain assets and scripts that are
//! executed as part of transactions.

use alloc::{collections::BTreeSet, string::ToString, vec::Vec};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{accounts::AccountId, crypto::rand::FeltRng};
use winter_maybe_async::{maybe_async, maybe_await};

use crate::{
    store::{InputNoteRecord, NoteFilter, OutputNoteRecord},
    Client, ClientError, IdPrefixFetchError,
};

pub mod script_roots;

mod import;
mod note_screener;

// RE-EXPORTS
// ================================================================================================
pub use miden_lib::notes::{create_p2id_note, create_p2idr_note, create_swap_note};
pub use miden_objects::{
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteFile, NoteId,
        NoteInclusionProof, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType,
        Nullifier,
    },
    NoteError,
};
pub use note_screener::{NoteConsumability, NoteRelevance, NoteScreener, NoteScreenerError};

// MIDEN CLIENT
// ================================================================================================

impl<R: FeltRng> Client<R> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Retrieves the input notes managed by the client from the store.
    ///
    /// # Errors
    ///
    /// Returns a [ClientError::StoreError] if the filter is [NoteFilter::Unique] and there is no
    /// Note with the provided ID
    #[maybe_async]
    pub fn get_input_notes(&self, filter: NoteFilter) -> Result<Vec<InputNoteRecord>, ClientError> {
        maybe_await!(self.store.get_input_notes(filter)).map_err(|err| err.into())
    }

    /// Returns the input notes and their consumability.
    ///
    /// If account_id is None then all consumable input notes are returned.
    #[maybe_async]
    pub fn get_consumable_notes(
        &self,
        account_id: Option<AccountId>,
    ) -> Result<Vec<(InputNoteRecord, Vec<NoteConsumability>)>, ClientError> {
        let commited_notes = maybe_await!(self.store.get_input_notes(NoteFilter::Committed))?;

        // For a committed note to be consumable its block header and MMR info must be tracked
        let unconsumable_committed_note_ids: BTreeSet<NoteId> =
            maybe_await!(self.store.get_notes_without_block_header())?
                .into_iter()
                .map(|note| note.id())
                .collect();

        let note_screener = NoteScreener::new(self.store.clone());

        let mut relevant_notes = Vec::new();
        for input_note in commited_notes {
            if unconsumable_committed_note_ids.contains(&input_note.id()) {
                continue;
            }

            let mut account_relevance =
                maybe_await!(note_screener.check_relevance(&input_note.clone().try_into()?))?;

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
    #[maybe_async]
    pub fn get_note_consumability(
        &self,
        note: InputNoteRecord,
    ) -> Result<Vec<NoteConsumability>, ClientError> {
        let note_screener = NoteScreener::new(self.store.clone());
        maybe_await!(note_screener.check_relevance(&note.clone().try_into()?))
            .map_err(|err| err.into())
    }

    /// Retrieves the input note given a [NoteId]
    ///
    /// # Errors
    ///
    /// Returns an error if there is no Note with the provided ID
    #[maybe_async]
    pub fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, ClientError> {
        Ok(maybe_await!(self.store.get_input_notes(NoteFilter::Unique(note_id)))?
            .pop()
            .expect("The vector always has one element for NoteFilter::Unique"))
    }

    // OUTPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns output notes managed by this client.
    #[maybe_async]
    pub fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, ClientError> {
        maybe_await!(self.store.get_output_notes(filter)).map_err(|err| err.into())
    }

    /// Returns the output note with the specified hash.
    #[maybe_async]
    pub fn get_output_note(&self, note_id: NoteId) -> Result<OutputNoteRecord, ClientError> {
        Ok(maybe_await!(self.store.get_output_notes(NoteFilter::Unique(note_id)))?
            .pop()
            .expect("The vector always has one element for NoteFilter::Unique"))
    }

    /// Compiles the provided program into a [NoteScript]
    pub fn compile_note_script(&self, note_script_ast: &str) -> Result<NoteScript, ClientError> {
        NoteScript::compile(note_script_ast, TransactionKernel::assembler())
            .map_err(ClientError::NoteError)
    }
}

/// Returns the client input note whose ID starts with `note_id_prefix`
///
/// # Errors
///
/// - Returns [IdPrefixFetchError::NoMatch] if we were unable to find any note where
///   `note_id_prefix` is a prefix of its id.
/// - Returns [IdPrefixFetchError::MultipleMatches] if there were more than one note found where
///   `note_id_prefix` is a prefix of its id.
#[maybe_async]
pub fn get_input_note_with_id_prefix<R: FeltRng>(
    client: &Client<R>,
    note_id_prefix: &str,
) -> Result<InputNoteRecord, IdPrefixFetchError> {
    let mut input_note_records = maybe_await!(client.get_input_notes(NoteFilter::All))
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
