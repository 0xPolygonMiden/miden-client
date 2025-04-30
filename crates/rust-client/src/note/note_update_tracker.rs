use alloc::collections::BTreeMap;

use miden_objects::{
    block::BlockHeader,
    note::{NoteId, NoteInclusionProof, Nullifier},
};

use crate::{
    ClientError,
    rpc::domain::{note::CommittedNote, nullifier::NullifierUpdate},
    store::{InputNoteRecord, OutputNoteRecord},
    transaction::{TransactionRecord, TransactionStatus},
};

// NOTE UPDATE
// ================================================================================================

/// Represents the possible types of updates that can be applied to a note in a
/// [`NoteUpdateTracker`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteUpdateType {
    /// Indicates that the note was already tracked but it was not updated.
    None,
    /// Indicates that the note is new and should be inserted in the store.
    Insert,
    /// Indicates that the note was already tracked and should be updated.
    Update,
}

/// Represents the possible states of an input note record in a [`NoteUpdateTracker`].
#[derive(Clone, Debug)]
pub struct InputNoteUpdate {
    /// Input note being updated.
    note: InputNoteRecord,
    /// Type of the note update.
    update_type: NoteUpdateType,
}

impl InputNoteUpdate {
    /// Creates a new [`InputNoteUpdate`] with the provided note with a `None` update type.
    fn new_none(note: InputNoteRecord) -> Self {
        Self { note, update_type: NoteUpdateType::None }
    }

    /// Creates a new [`InputNoteUpdate`] with the provided note with an `Insert` update type.
    fn new_insert(note: InputNoteRecord) -> Self {
        Self {
            note,
            update_type: NoteUpdateType::Insert,
        }
    }

    /// Creates a new [`InputNoteUpdate`] with the provided note with an `Update` update type.
    fn new_update(note: InputNoteRecord) -> Self {
        Self {
            note,
            update_type: NoteUpdateType::Update,
        }
    }

    /// Returns a reference the inner note record.
    pub fn inner(&self) -> &InputNoteRecord {
        &self.note
    }

    /// Returns a mutable reference to the inner note record. If the u
    fn inner_mut(&mut self) -> &mut InputNoteRecord {
        self.update_type = match self.update_type {
            NoteUpdateType::None | NoteUpdateType::Update => NoteUpdateType::Update,
            NoteUpdateType::Insert => NoteUpdateType::Insert,
        };

        &mut self.note
    }

    /// Returns the type of the note update.
    pub fn update_type(&self) -> &NoteUpdateType {
        &self.update_type
    }
}

/// Represents the possible states of an output note record in a [`NoteUpdateTracker`].
#[derive(Clone, Debug)]
pub struct OutputNoteUpdate {
    /// Output note being updated.
    note: OutputNoteRecord,
    /// Type of the note update.
    update_type: NoteUpdateType,
}

impl OutputNoteUpdate {
    /// Creates a new [`OutputNoteUpdate`] with the provided note with a `None` update type.
    fn new_none(note: OutputNoteRecord) -> Self {
        Self { note, update_type: NoteUpdateType::None }
    }

    /// Creates a new [`OutputNoteUpdate`] with the provided note with an `Insert` update type.
    fn new_insert(note: OutputNoteRecord) -> Self {
        Self {
            note,
            update_type: NoteUpdateType::Insert,
        }
    }

    /// Returns a reference the inner note record.
    pub fn inner(&self) -> &OutputNoteRecord {
        &self.note
    }

    /// Returns a mutable reference to the inner note record. If the update type is `None` or
    /// `Update`, it will be set to `Update`.
    fn inner_mut(&mut self) -> &mut OutputNoteRecord {
        self.update_type = match self.update_type {
            NoteUpdateType::None | NoteUpdateType::Update => NoteUpdateType::Update,
            NoteUpdateType::Insert => NoteUpdateType::Insert,
        };

        &mut self.note
    }

    /// Returns the type of the note update.
    pub fn update_type(&self) -> &NoteUpdateType {
        &self.update_type
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
    /// Creates a [`NoteUpdateTracker`] with already-tracked notes.
    pub fn new(
        input_notes: impl IntoIterator<Item = InputNoteRecord>,
        output_notes: impl IntoIterator<Item = OutputNoteRecord>,
    ) -> Self {
        Self {
            input_notes: input_notes
                .into_iter()
                .map(|note| (note.id(), InputNoteUpdate::new_none(note)))
                .collect(),
            output_notes: output_notes
                .into_iter()
                .map(|note| (note.id(), OutputNoteUpdate::new_none(note)))
                .collect(),
        }
    }

    /// Creates a [`NoteUpdateTracker`] for updates related to transactions.
    ///
    /// A transaction can:
    ///
    /// - Create input notes
    /// - Update existing input notes (by consuming them)
    /// - Create output notes
    pub fn for_transaction_updates(
        new_input_notes: impl IntoIterator<Item = InputNoteRecord>,
        updated_input_notes: impl IntoIterator<Item = InputNoteRecord>,
        new_output_notes: impl IntoIterator<Item = OutputNoteRecord>,
    ) -> Self {
        Self {
            input_notes: new_input_notes
                .into_iter()
                .map(|note| (note.id(), InputNoteUpdate::new_insert(note)))
                .chain(
                    updated_input_notes
                        .into_iter()
                        .map(|note| (note.id(), InputNoteUpdate::new_update(note))),
                )
                .collect(),
            output_notes: new_output_notes
                .into_iter()
                .map(|note| (note.id(), OutputNoteUpdate::new_insert(note)))
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
        self.input_notes.values().filter(|note| {
            matches!(note.update_type, NoteUpdateType::Insert | NoteUpdateType::Update)
        })
    }

    /// Returns all output note records that have been updated.
    ///
    /// This may include:
    /// - New notes that have been created that should be inserted.
    /// - Existing tracked notes that should be updated.
    pub fn updated_output_notes(&self) -> impl Iterator<Item = &OutputNoteUpdate> {
        self.output_notes.values().filter(|note| {
            matches!(note.update_type, NoteUpdateType::Insert | NoteUpdateType::Update)
        })
    }

    /// Returns whether no new note-related information has been retrieved.
    pub fn is_empty(&self) -> bool {
        self.input_notes.is_empty() && self.output_notes.is_empty()
    }

    pub fn unspent_nullifiers(&self) -> impl Iterator<Item = Nullifier> + '_ {
        self.input_notes
            .values()
            .filter(|note| !note.inner().is_consumed())
            .map(|note| note.inner().nullifier())
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
                .insert(input_note_record.id(), InputNoteUpdate::new_insert(input_note_record));
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
    pub(crate) fn apply_nullifiers_state_transitions<'a>(
        &mut self,
        nullifier_update: &NullifierUpdate,
        mut committed_transactions: impl Iterator<Item = &'a TransactionRecord>,
    ) -> Result<(), ClientError> {
        if let Some(input_note_record) =
            self.get_input_note_by_nullifier(nullifier_update.nullifier)
        {
            if let Some(consumer_transaction) = committed_transactions
                .find(|t| input_note_record.consumer_transaction_id() == Some(&t.id))
            {
                // The note was being processed by a local transaction that just got committed
                if let TransactionStatus::Committed(commit_height) = consumer_transaction.status {
                    input_note_record
                        .transaction_committed(consumer_transaction.id, commit_height.as_u32())?;
                }
            } else {
                // The note was consumed by an external transaction
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

        Ok(())
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
