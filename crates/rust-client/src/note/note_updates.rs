use alloc::collections::{BTreeMap, BTreeSet};

use miden_objects::{
    block::BlockHeader,
    note::{NoteId, NoteInclusionProof, Nullifier},
    transaction::TransactionId,
};

use crate::{
    rpc::domain::{
        note::CommittedNote, nullifier::NullifierUpdate, transaction::TransactionUpdate,
    },
    store::{InputNoteRecord, OutputNoteRecord},
    sync::NoteTagRecord,
    ClientError,
};

// NOTE UPDATES
// ================================================================================================

/// Contains note changes to apply to the store.
#[derive(Clone, Debug, Default)]
pub struct NoteUpdates {
    /// A map of new and updated input note records to be upserted in the store.
    updated_input_notes: BTreeMap<NoteId, InputNoteRecord>,
    /// A map of updated output note records to be upserted in the store.
    updated_output_notes: BTreeMap<NoteId, OutputNoteRecord>,
}

impl NoteUpdates {
    /// Creates a [`NoteUpdates`].
    pub fn new(
        updated_input_notes: impl IntoIterator<Item = InputNoteRecord>,
        updated_output_notes: impl IntoIterator<Item = OutputNoteRecord>,
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

    // GETTERS
    // --------------------------------------------------------------------------------------------

    /// Returns all input note records that have been updated.
    /// This may include:
    /// - New notes that have been created that should be inserted.
    /// - Existing tracked notes that should be updated.
    pub fn updated_input_notes(&self) -> impl Iterator<Item = &InputNoteRecord> {
        self.updated_input_notes.values()
    }

    /// Returns all output note records that have been updated.
    /// This may include:
    /// - New notes that have been created that should be inserted.
    /// - Existing tracked notes that should be updated.
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

    /// Returns the tags of all notes that need to be removed from the store after the state sync.
    /// These are the tags of notes that have been committed and no longer need to be tracked.
    pub fn tags_to_remove(&self) -> impl Iterator<Item = NoteTagRecord> + '_ {
        self.committed_input_notes().map(|note| {
            NoteTagRecord::with_note_source(
                note.metadata().expect("Committed notes should have metadata").tag(),
                note.id(),
            )
        })
    }

    /// Returns the IDs of all notes that have been committed in this update.
    /// This includes both new notes and tracked expected notes that were committed in this update.
    pub fn committed_note_ids(&self) -> BTreeSet<NoteId> {
        let committed_output_note_ids = self
            .updated_output_notes
            .values()
            .filter_map(|note_record| note_record.is_committed().then_some(note_record.id()));

        let committed_input_note_ids = self
            .updated_input_notes
            .values()
            .filter_map(|note_record| note_record.is_committed().then_some(note_record.id()));

        committed_input_note_ids
            .chain(committed_output_note_ids)
            .collect::<BTreeSet<_>>()
    }

    /// Returns the IDs of all notes that have been consumed.
    /// This includes both notes that have been consumed locally or externally in this update.
    pub fn consumed_note_ids(&self) -> BTreeSet<NoteId> {
        let consumed_output_note_ids = self
            .updated_output_notes
            .values()
            .filter_map(|note_record| note_record.is_consumed().then_some(note_record.id()));

        let consumed_input_note_ids = self
            .updated_input_notes
            .values()
            .filter_map(|note_record| note_record.is_consumed().then_some(note_record.id()));

        consumed_input_note_ids.chain(consumed_output_note_ids).collect::<BTreeSet<_>>()
    }

    // UPDATE METHODS
    // --------------------------------------------------------------------------------------------

    /// Inserts new or updated input and output notes. If an update with the same note ID already
    /// exists, it will be replaced.
    pub(crate) fn insert_updates(
        &mut self,
        input_note: Option<InputNoteRecord>,
        output_note: Option<OutputNoteRecord>,
    ) {
        if let Some(input_note) = input_note {
            self.updated_input_notes.insert(input_note.id(), input_note);
        }
        if let Some(output_note) = output_note {
            self.updated_output_notes.insert(output_note.id(), output_note);
        }
    }

    /// Applies the necessary state transitions to the [`NoteUpdates`] when a note is committed in a
    /// block.
    pub(crate) fn apply_committed_note_state_transitions(
        &mut self,
        committed_note: &CommittedNote,
        block_header: &BlockHeader,
    ) -> Result<(), ClientError> {
        let inclusion_proof = NoteInclusionProof::new(
            block_header.block_num(),
            committed_note.note_index(),
            committed_note.merkle_path().clone(),
        )?;

        if let Some(input_note_record) = self.get_input_note_by_id(*committed_note.note_id()) {
            // The note belongs to our locally tracked set of input notes
            input_note_record
                .inclusion_proof_received(inclusion_proof.clone(), committed_note.metadata())?;
            input_note_record.block_header_received(block_header)?;
        }

        if let Some(output_note_record) = self.get_output_note_by_id(*committed_note.note_id()) {
            // The note belongs to our locally tracked set of output notes
            output_note_record.inclusion_proof_received(inclusion_proof.clone())?;
        }

        Ok(())
    }

    /// Applies the necessary state transitions to the [`NoteUpdates`] when a note is nullified in a
    /// block. For input note records two possible scenarios are considered:
    /// 1. The note was being processed by a local transaction that just got committed.
    /// 2. The note was consumed by an external transaction. If a local transaction was processing
    ///    the note and it didn't get committed, the transaction should be discarded.
    pub(crate) fn apply_nullifiers_state_transitions(
        &mut self,
        nullifier_update: &NullifierUpdate,
        transaction_updates: &[TransactionUpdate],
    ) -> Result<Option<TransactionId>, ClientError> {
        let mut discarded_transaction = None;

        if let Some(input_note_record) =
            self.get_input_note_by_nullifier(nullifier_update.nullifier)
        {
            if let Some(consumer_transaction) = transaction_updates
                .iter()
                .find(|t| input_note_record.consumer_transaction_id() == Some(&t.transaction_id))
            {
                // The note was being processed by a local transaction that just got committed
                input_note_record.transaction_committed(
                    consumer_transaction.transaction_id,
                    consumer_transaction.block_num,
                )?;
            } else {
                // The note was consumed by an external transaction
                if let Some(id) = input_note_record.consumer_transaction_id() {
                    // The note was being processed by a local transaction that didn't end up being
                    // committed so it should be discarded
                    discarded_transaction.replace(*id);
                }
                input_note_record
                    .consumed_externally(nullifier_update.nullifier, nullifier_update.block_num)?;
            }
        }

        if let Some(output_note_record) =
            self.get_output_note_by_nullifier(nullifier_update.nullifier)
        {
            output_note_record
                .nullifier_received(nullifier_update.nullifier, nullifier_update.block_num)?;
        }

        Ok(discarded_transaction)
    }

    // PRIVATE HELPERS
    // --------------------------------------------------------------------------------------------

    /// Returns a mutable reference to the input note record with the provided ID if it exists.
    fn get_input_note_by_id(&mut self, note_id: NoteId) -> Option<&mut InputNoteRecord> {
        self.updated_input_notes.get_mut(&note_id)
    }

    /// Returns a mutable reference to the output note record with the provided ID if it exists.
    fn get_output_note_by_id(&mut self, note_id: NoteId) -> Option<&mut OutputNoteRecord> {
        self.updated_output_notes.get_mut(&note_id)
    }

    /// Returns a mutable reference to the input note record with the provided nullifier if it
    /// exists.
    fn get_input_note_by_nullifier(
        &mut self,
        nullifier: Nullifier,
    ) -> Option<&mut InputNoteRecord> {
        self.updated_input_notes.values_mut().find(|note| note.nullifier() == nullifier)
    }

    /// Returns a mutable reference to the output note record with the provided nullifier if it
    /// exists.
    fn get_output_note_by_nullifier(
        &mut self,
        nullifier: Nullifier,
    ) -> Option<&mut OutputNoteRecord> {
        self.updated_output_notes
            .values_mut()
            .find(|note| note.nullifier() == Some(nullifier))
    }
}
