use alloc::collections::BTreeMap;

use miden_objects::{
    block::BlockHeader,
    note::{NoteId, NoteInclusionProof, Nullifier},
    transaction::TransactionId,
};

use crate::{
    ClientError,
    rpc::domain::{
        note::CommittedNote, nullifier::NullifierUpdate, transaction::TransactionUpdate,
    },
    store::{InputNoteRecord, OutputNoteRecord},
    sync::NoteTagRecord,
};

/// NOTE UPDATE ENUMS
/// ================================================================================================

/// Represents the possible states of an input note record in a [`NoteUpdateTracker`].
#[derive(Clone, Debug)]
pub enum InputNoteUpdate {
    /// Indicates that the note was already tracked but it was not updated.
    None(InputNoteRecord),
    /// Indicates that the note is new and should be inserted in the store.
    Insert(InputNoteRecord),
    /// Indicates that the note was already tracked and should be updated.
    Update(InputNoteRecord),
}

impl InputNoteUpdate {
    /// Returns a reference the inner note record.
    pub fn inner(&self) -> &InputNoteRecord {
        match self {
            InputNoteUpdate::None(note)
            | InputNoteUpdate::Insert(note)
            | InputNoteUpdate::Update(note) => note,
        }
    }

    /// Returns a mutable reference to the inner note record.
    fn inner_mut(&mut self) -> &mut InputNoteRecord {
        match self {
            InputNoteUpdate::None(note)
            | InputNoteUpdate::Insert(note)
            | InputNoteUpdate::Update(note) => note,
        }
    }
}

/// Represents the possible states of an output note record in a [`NoteUpdateTracker`].
#[derive(Clone, Debug)]
pub enum OutputNoteUpdate {
    /// Indicates that the note was already tracked but it was not updated.
    None(OutputNoteRecord),
    /// Indicates that the note is new and should be inserted in the store.
    Insert(OutputNoteRecord),
    /// Indicates that the note was already tracked and should be updated.
    Update(OutputNoteRecord),
}

impl OutputNoteUpdate {
    /// Returns a reference the inner note record.
    pub fn inner(&self) -> &OutputNoteRecord {
        match self {
            OutputNoteUpdate::None(note)
            | OutputNoteUpdate::Insert(note)
            | OutputNoteUpdate::Update(note) => note,
        }
    }

    /// Returns a mutable reference to the inner note record.
    fn inner_mut(&mut self) -> &mut OutputNoteRecord {
        match self {
            OutputNoteUpdate::None(note)
            | OutputNoteUpdate::Insert(note)
            | OutputNoteUpdate::Update(note) => note,
        }
    }
}

// NOTE UPDATE TRACKER
// ================================================================================================

/// Contains note changes to apply to the store.
///
/// This includes new notes that have been created and existing notes that have been updated. The
/// tracker also lets state changes be applied to the contained notes, this allows for already
/// updated notes to be further updated as new information is received.
#[derive(Clone, Debug, Default)]
pub struct NoteUpdateTracker {
    /// A map of new and updated input note records to be upserted in the store.
    input_notes: BTreeMap<NoteId, InputNoteUpdate>,
    /// A map of updated output note records to be upserted in the store.
    output_notes: BTreeMap<NoteId, OutputNoteUpdate>,
}

impl NoteUpdateTracker {
    /// Creates a [`NoteUpdateTracker`].
    pub fn new(
        input_notes: impl IntoIterator<Item = InputNoteRecord>,
        output_notes: impl IntoIterator<Item = OutputNoteRecord>,
    ) -> Self {
        Self {
            input_notes: input_notes
                .into_iter()
                .map(|note| (note.id(), InputNoteUpdate::None(note)))
                .collect(),
            output_notes: output_notes
                .into_iter()
                .map(|note| (note.id(), OutputNoteUpdate::None(note)))
                .collect(),
        }
    }

    // GETTERS
    // --------------------------------------------------------------------------------------------

    /// Returns all input note records that have been updated.
    ///
    /// This may include:
    /// - New notes that have been created that should be inserted.
    /// - Existing tracked notes that should be updated.
    pub fn updated_input_notes(&self) -> impl Iterator<Item = &InputNoteUpdate> {
        self.input_notes
            .values()
            .filter(|note| matches!(note, InputNoteUpdate::Insert(_) | InputNoteUpdate::Update(_)))
    }

    /// Returns all output note records that have been updated.
    ///
    /// This may include:
    /// - New notes that have been created that should be inserted.
    /// - Existing tracked notes that should be updated.
    pub fn updated_output_notes(&self) -> impl Iterator<Item = &OutputNoteUpdate> {
        self.output_notes.values().filter(|note| {
            matches!(note, OutputNoteUpdate::Insert(_) | OutputNoteUpdate::Update(_))
        })
    }

    /// Returns whether no new note-related information has been retrieved.
    pub fn is_empty(&self) -> bool {
        self.input_notes.is_empty() && self.output_notes.is_empty()
    }

    /// Returns the tags of all notes that need to be removed from the store after the state sync.
    ///
    /// These are the tags of notes that have been committed and no longer need to be tracked.
    pub fn tags_to_remove(&self) -> impl Iterator<Item = NoteTagRecord> + '_ {
        self.input_notes
            .values()
            .filter(|note| note.inner().is_committed())
            .map(|note| {
                NoteTagRecord::with_note_source(
                    note.inner().metadata().expect("Committed notes should have metadata").tag(),
                    note.inner().id(),
                )
            })
    }

    // UPDATE METHODS
    // --------------------------------------------------------------------------------------------

    /// Applies the necessary state transitions to the [`NoteUpdateTracker`] when a note is
    /// committed in a block.
    pub(crate) fn apply_committed_note_state_transitions(
        &mut self,
        committed_note: &CommittedNote,
        public_note_data: Option<InputNoteRecord>,
        block_header: &BlockHeader,
    ) -> Result<(), ClientError> {
        let inclusion_proof = NoteInclusionProof::new(
            block_header.block_num(),
            committed_note.note_index(),
            committed_note.merkle_path().clone(),
        )?;

        if let Some(mut input_note_record) = public_note_data {
            input_note_record.block_header_received(block_header)?;
            self.input_notes
                .insert(input_note_record.id(), InputNoteUpdate::Insert(input_note_record));
        }

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

    /// Applies the necessary state transitions to the [`NoteUpdateTracker`] when a note is
    /// nullified in a block.
    ///
    /// For input note records two possible scenarios are considered:
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
        self.input_notes.get_mut(&note_id).map(InputNoteUpdate::inner_mut)
    }

    /// Returns a mutable reference to the output note record with the provided ID if it exists.
    fn get_output_note_by_id(&mut self, note_id: NoteId) -> Option<&mut OutputNoteRecord> {
        self.output_notes.get_mut(&note_id).map(OutputNoteUpdate::inner_mut)
    }

    /// Returns a mutable reference to the input note record with the provided nullifier if it
    /// exists.
    fn get_input_note_by_nullifier(
        &mut self,
        nullifier: Nullifier,
    ) -> Option<&mut InputNoteRecord> {
        self.input_notes
            .values_mut()
            .find(|note| note.inner().nullifier() == nullifier)
            .map(InputNoteUpdate::inner_mut)
    }

    /// Returns a mutable reference to the output note record with the provided nullifier if it
    /// exists.
    fn get_output_note_by_nullifier(
        &mut self,
        nullifier: Nullifier,
    ) -> Option<&mut OutputNoteRecord> {
        self.output_notes
            .values_mut()
            .find(|note| note.inner().nullifier() == Some(nullifier))
            .map(OutputNoteUpdate::inner_mut)
    }
}
