use alloc::{collections::BTreeSet, string::ToString, vec::Vec};

use miden_objects::{accounts::AccountId, assembly::ProgramAst, crypto::rand::FeltRng};
use miden_tx::{auth::TransactionAuthenticator, ScriptTarget};
use tracing::info;
use winter_maybe_async::{maybe_async, maybe_await};

use crate::{
    rpc::{NodeRpcClient, NoteDetails},
    store::{InputNoteRecord, NoteFilter, NoteStatus, OutputNoteRecord, Store, StoreError},
    Client, ClientError, IdPrefixFetchError,
};

mod note_screener;

// RE-EXPORTS
// ================================================================================================

pub use miden_objects::{
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteFile, NoteId, NoteInclusionProof, NoteInputs,
        NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType, Nullifier,
    },
    NoteError,
};
pub(crate) use note_screener::NoteScreener;
pub use note_screener::{NoteConsumability, NoteRelevance, NoteScreenerError};

// MIDEN CLIENT
// ================================================================================================

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    #[maybe_async]
    pub fn get_input_notes(
        &self,
        filter: NoteFilter<'_>,
    ) -> Result<Vec<InputNoteRecord>, ClientError> {
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

        // For a committed note to be consumable its block header and mmr info must be tracked
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

    /// Returns the input note with the specified hash.
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
        filter: NoteFilter<'_>,
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

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store. The information stored depends on the
    /// type of note file provided.
    ///
    /// If the note file is a [NoteFile::NoteId], the note is fecthed from the node and stored in
    /// the client's store. If the note is private or does not exist, an error is returned. If the
    /// ID was already stored, the inclusion proof and metadata are updated.
    /// If the note file is a [NoteFile::NoteDetails], a new note is created with the provided
    /// details. The note is marked as ignored if it contains no tag or if the tag is not relevant.
    /// If the note file is a [NoteFile::NoteWithProof], the note is stored with the provided
    /// inclusion proof and metadata. The MMR data is not fetched from the node.
    pub async fn import_note(&mut self, note_file: NoteFile) -> Result<NoteId, ClientError> {
        let note = match note_file {
            NoteFile::NoteId(id) => {
                let mut chain_notes = self.rpc_api.get_notes_by_id(&[id]).await?;
                if chain_notes.is_empty() {
                    return Err(ClientError::ExistenceVerificationError(id));
                }

                let note_details =
                    chain_notes.pop().expect("chain_notes should have at least one element");

                let inclusion_details = note_details.inclusion_details();

                // Add the inclusion proof to the imported note
                let inclusion_proof = NoteInclusionProof::new(
                    inclusion_details.block_num,
                    inclusion_details.note_index,
                    inclusion_details.merkle_path.clone(),
                )?;

                let tracked_note = maybe_await!(self.get_input_note(id));

                if let Err(ClientError::StoreError(StoreError::NoteNotFound(_))) = tracked_note {
                    let node_note = match note_details {
                        NoteDetails::Public(note, _) => note,
                        NoteDetails::OffChain(..) => {
                            return Err(ClientError::NoteImportError(
                                "Incomplete imported note is private".to_string(),
                            ))
                        },
                    };

                    // If note is not tracked, we create a new one.
                    let details = node_note.clone().into();

                    InputNoteRecord::new(
                        node_note.id(),
                        node_note.recipient().digest(),
                        node_note.assets().clone(),
                        NoteStatus::Committed {
                            block_height: inclusion_proof.location().block_num() as u64,
                        },
                        Some(*node_note.metadata()),
                        Some(inclusion_proof),
                        details,
                        false,
                        None,
                    )
                } else {
                    // If note is already tracked, we update the inclusion proof and metadata.
                    let tracked_note = tracked_note?;

                    // TODO: Join these calls to one method that updates both fields with one query (issue #404)
                    maybe_await!(self
                        .store
                        .update_note_inclusion_proof(tracked_note.id(), inclusion_proof))?;
                    maybe_await!(self
                        .store
                        .update_note_metadata(tracked_note.id(), *note_details.metadata()))?;

                    return Ok(tracked_note.id());
                }
            },
            NoteFile::NoteDetails(details, None) => {
                let record_details = details.clone().into();

                InputNoteRecord::new(
                    details.id(),
                    details.recipient().digest(),
                    details.assets().clone(),
                    NoteStatus::Expected { created_at: 0 },
                    None,
                    None,
                    record_details,
                    true,
                    None,
                )
            },
            NoteFile::NoteDetails(details, Some(tag)) => {
                let tracked_tags = maybe_await!(self.get_note_tags())?;

                let account_tags = maybe_await!(self.get_account_stubs())?
                    .into_iter()
                    .map(|(stub, _)| NoteTag::from_account_id(stub.id(), NoteExecutionHint::Local))
                    .collect::<Result<Vec<_>, _>>()?;

                let uncommited_note_tags =
                    maybe_await!(self.get_input_notes(NoteFilter::Expected))?
                        .into_iter()
                        .filter_map(|note| note.metadata().map(|metadata| metadata.tag()))
                        .collect::<Vec<_>>();

                let ignored =
                    ![tracked_tags, account_tags, uncommited_note_tags].concat().contains(&tag);

                if ignored {
                    info!("Ignoring note with tag {}", tag);
                }

                let record_details = details.clone().into();

                InputNoteRecord::new(
                    details.id(),
                    details.recipient().digest(),
                    details.assets().clone(),
                    NoteStatus::Expected { created_at: 0 },
                    None,
                    None,
                    record_details,
                    ignored,
                    Some(tag),
                )
            },
            NoteFile::NoteWithProof(note, inclusion_proof) => {
                let details = note.clone().into();

                InputNoteRecord::new(
                    note.id(),
                    note.recipient().digest(),
                    note.assets().clone(),
                    NoteStatus::Committed {
                        block_height: inclusion_proof.location().block_num() as u64,
                    },
                    Some(*note.metadata()),
                    Some(inclusion_proof),
                    details,
                    false,
                    None,
                )
            },
        };
        let id = note.id();

        maybe_await!(self.store.insert_input_note(note))?;
        Ok(id)
    }

    /// Compiles the provided program into a [NoteScript] and checks (to the extent possible) if
    /// the specified note program could be executed against all accounts with the specified
    /// interfaces.
    pub fn compile_note_script(
        &self,
        note_script_ast: ProgramAst,
        target_account_procs: Vec<ScriptTarget>,
    ) -> Result<NoteScript, ClientError> {
        self.tx_executor
            .compile_note_script(note_script_ast, target_account_procs)
            .map_err(ClientError::TransactionExecutorError)
    }
}

/// Returns the client input note whose ID starts with `note_id_prefix`
///
/// # Errors
///
/// - Returns [IdPrefixFetchError::NoMatch] if we were unable to find any note where
///   `note_id_prefix` is a prefix of its id.
/// - Returns [IdPrefixFetchError::MultipleMatches] if there were more than one note found
///   where `note_id_prefix` is a prefix of its id.
#[maybe_async]
pub fn get_input_note_with_id_prefix<
    N: NodeRpcClient,
    R: FeltRng,
    S: Store,
    A: TransactionAuthenticator,
>(
    client: &Client<N, R, S, A>,
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
