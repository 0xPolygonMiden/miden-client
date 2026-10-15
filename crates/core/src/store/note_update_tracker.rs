use alloc::collections::BTreeMap;

use miden_protocol::block::BlockNumber;
use miden_protocol::note::{NoteDetailsCommitment, NoteId, NoteMetadata, Nullifier};
use miden_protocol::transaction::TransactionId;

use crate::errors::NoteRecordError;
use crate::store::{InputNoteRecord, OutputNoteRecord};

// NOTE UPDATE
// ================================================================================================

/// Represents the possible types of updates that can be applied to a note in a
/// [`NoteUpdateTracker`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NoteUpdateType {
    /// Indicates that the note was already tracked but it was not updated.
    None = 0,
    /// Indicates that the note is new and should be inserted in the store.
    Insert = 1,
    /// Indicates that the note was already tracked and should be updated.
    Update = 2,
    /// Indicates that a previously-tracked metadata-less (`Expected`) note has just been committed.
    /// It must be persisted as a full-row insert (like [`Self::Insert`]) so its now-known `note_id`
    /// and `nullifier` columns are written, but for reporting it is a *committed* tracked note —
    /// not a newly-discovered one — so it is summarized under committed notes, not new notes.
    InsertCommitted = 3,
}

impl NoteUpdateType {
    /// Whether this update carries a pending store write, as opposed to a note that was merely
    /// loaded as already-tracked context ([`Self::None`]). True for [`Self::Insert`],
    /// [`Self::Update`], and [`Self::InsertCommitted`].
    pub fn is_modified(self) -> bool {
        matches!(self, Self::Insert | Self::Update | Self::InsertCommitted)
    }
}

impl TryFrom<u8> for NoteUpdateType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(NoteUpdateType::None),
            1 => Ok(NoteUpdateType::Insert),
            2 => Ok(NoteUpdateType::Update),
            3 => Ok(NoteUpdateType::InsertCommitted),
            other => Err(other),
        }
    }
}

/// Represents the possible states of an input note record in a [`NoteUpdateTracker`].
#[derive(Clone, Debug, PartialEq)]
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

    /// Creates a new [`InputNoteUpdate`] for a previously-tracked expected note that has just been
    /// committed (see [`NoteUpdateType::InsertCommitted`]).
    fn new_insert_committed(note: InputNoteRecord) -> Self {
        Self {
            note,
            update_type: NoteUpdateType::InsertCommitted,
        }
    }

    /// Returns a reference the inner note record.
    pub fn inner(&self) -> &InputNoteRecord {
        &self.note
    }

    /// Returns a mutable reference to the inner note record. If the update type is `None` or
    /// `Update`, it will be set to `Update`; insert-typed updates keep their type.
    pub fn inner_mut(&mut self) -> &mut InputNoteRecord {
        self.update_type = match self.update_type {
            NoteUpdateType::None | NoteUpdateType::Update => NoteUpdateType::Update,
            NoteUpdateType::Insert => NoteUpdateType::Insert,
            NoteUpdateType::InsertCommitted => NoteUpdateType::InsertCommitted,
        };

        &mut self.note
    }

    /// Returns the type of the note update.
    pub fn update_type(&self) -> &NoteUpdateType {
        &self.update_type
    }

    /// Returns the identifier of the inner note. Returns `None` when the underlying
    /// [`InputNoteRecord`] has no metadata (see [`InputNoteRecord::id`]).
    pub fn id(&self) -> Option<NoteId> {
        self.note.id()
    }

    /// Returns the per-account position of the consuming transaction within the account's execution
    /// chain for the block. `None` for non-consumed notes or when the order has not been determined
    /// yet.
    pub fn consumed_tx_order(&self) -> Option<u32> {
        self.note.state().consumed_tx_order()
    }
}

/// Represents the possible states of an output note record in a [`NoteUpdateTracker`].
#[derive(Clone, Debug, PartialEq)]
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

    /// Creates a new [`OutputNoteUpdate`] with the provided note with an `Update` update type.
    fn new_update(note: OutputNoteRecord) -> Self {
        Self {
            note,
            update_type: NoteUpdateType::Update,
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
            // Output notes are never assigned `InsertCommitted` (it is input-note specific), but
            // the match must be exhaustive; treat it as an insert.
            NoteUpdateType::Insert | NoteUpdateType::InsertCommitted => NoteUpdateType::Insert,
        };

        &mut self.note
    }

    /// Returns the type of the note update.
    pub fn update_type(&self) -> &NoteUpdateType {
        &self.update_type
    }

    /// Returns the identifier of the inner note.
    pub fn id(&self) -> NoteId {
        self.note.id()
    }
}

// NOTE UPDATE TRACKER
// ================================================================================================

/// Contains note changes to apply to the store.
///
/// This includes new notes that have been created and existing notes that have been updated. The
/// tracker also lets state changes be applied to the contained notes, this allows for already
/// updated notes to be further updated as new information is received.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NoteUpdateTracker {
    /// All new and updated input note records to be upserted in the store, keyed by their details
    /// commitment. The details commitment is metadata-independent and therefore always available,
    /// including for metadata-less notes (e.g. expected notes imported from bare details, or future
    /// notes created by a transaction) that do not yet have a `NoteId`.
    input_notes: BTreeMap<NoteDetailsCommitment, InputNoteUpdate>,
    /// A map of updated output note records to be upserted in the store.
    output_notes: BTreeMap<NoteId, OutputNoteUpdate>,
    /// Lookup index from nullifier to the details commitment of the input note. Only populated for
    /// metadata-bearing notes, as a metadata-less note has no nullifier.
    input_notes_by_nullifier: BTreeMap<Nullifier, NoteDetailsCommitment>,
    /// Lookup index from `NoteId` to the details commitment of the input note. Only populated for
    /// metadata-bearing notes, since a metadata-less note has no `NoteId`. Lets a note be found by
    /// its id even though `input_notes` is keyed by details commitment.
    input_notes_by_id: BTreeMap<NoteId, NoteDetailsCommitment>,
    /// Fast lookup map from nullifier to output note id.
    output_notes_by_nullifier: BTreeMap<Nullifier, NoteId>,
    /// Map from nullifier to its per-account position in the consuming transaction order.
    /// Nullifiers from the same account are in execution order; ordering across different accounts
    /// is not guaranteed.
    nullifier_order: BTreeMap<Nullifier, u32>,
}

impl NoteUpdateTracker {
    /// Creates a [`NoteUpdateTracker`] with already-tracked notes.
    pub fn new(
        input_notes: impl IntoIterator<Item = InputNoteRecord>,
        output_notes: impl IntoIterator<Item = OutputNoteRecord>,
    ) -> Self {
        let mut tracker = Self::default();
        for note in input_notes {
            tracker.insert_input_note(note, NoteUpdateType::None);
        }
        for note in output_notes {
            tracker.insert_output_note(note, NoteUpdateType::None);
        }

        tracker
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
        let mut tracker = Self::default();

        for note in new_input_notes {
            tracker.insert_input_note(note, NoteUpdateType::Insert);
        }

        for note in updated_input_notes {
            tracker.insert_input_note(note, NoteUpdateType::Update);
        }

        for note in new_output_notes {
            tracker.insert_output_note(note, NoteUpdateType::Insert);
        }

        tracker
    }

    // GETTERS
    // --------------------------------------------------------------------------------------------

    /// Returns all input note records that have been updated.
    ///
    /// This may include:
    /// - New notes that have been created that should be inserted.
    /// - Existing tracked notes that should be updated.
    ///
    /// Metadata-less expected notes (e.g. future notes created by a transaction, such as swap
    /// payback notes) are included as well: they have no `NoteId` yet but must still be persisted
    /// and have their tags registered. The `update_type` filter ensures notes merely loaded as
    /// already-tracked context (`NoteUpdateType::None`) are not re-emitted.
    pub fn updated_input_notes(&self) -> impl Iterator<Item = &InputNoteUpdate> {
        self.input_notes.values().filter(|note| note.update_type.is_modified())
    }

    /// Returns the ids of updated input notes that are now consumed. `input_notes` is keyed by
    /// details commitment, so the `input_notes_by_id` index provides each note's `NoteId`.
    pub fn consumed_input_note_ids(&self) -> impl Iterator<Item = NoteId> + '_ {
        self.input_notes_by_id.iter().filter_map(|(note_id, commitment)| {
            let update = self.input_notes.get(commitment)?;
            (update.update_type.is_modified() && update.inner().is_consumed()).then_some(*note_id)
        })
    }

    /// `NoteId`s of every input + output note that transitioned to a consumed state this sync.
    /// These are confirmed consumptions reflected in the tracker, not raw nullifier-prefix hits.
    pub fn consumed_note_ids(&self) -> impl Iterator<Item = NoteId> + '_ {
        let output = self.output_notes.iter().filter_map(|(note_id, update)| {
            (update.update_type.is_modified() && update.inner().is_consumed()).then_some(*note_id)
        });
        self.consumed_input_note_ids().chain(output)
    }

    /// Returns all output note records that have been updated.
    ///
    /// This may include:
    /// - New notes that have been created that should be inserted.
    /// - Existing tracked notes that should be updated.
    pub fn updated_output_notes(&self) -> impl Iterator<Item = &OutputNoteUpdate> {
        self.output_notes.values().filter(|note| note.update_type.is_modified())
    }

    /// Returns whether no new note-related information has been retrieved.
    pub fn is_empty(&self) -> bool {
        self.input_notes.is_empty() && self.output_notes.is_empty()
    }

    /// Returns input and output note unspent nullifiers.
    pub fn unspent_nullifiers(&self) -> impl Iterator<Item = Nullifier> {
        let input_note_unspent_nullifiers = self
            .input_notes
            .values()
            .filter(|note| !note.inner().is_consumed())
            .filter_map(|note| note.inner().nullifier());

        let output_note_unspent_nullifiers = self
            .output_notes
            .values()
            .filter(|note| !note.inner().is_consumed())
            .filter_map(|note| note.inner().nullifier());

        input_note_unspent_nullifiers.chain(output_note_unspent_nullifiers)
    }

    /// Returns the block numbers containing input notes that remain unspent after this update.
    pub fn unspent_input_note_block_numbers(&self) -> impl Iterator<Item = BlockNumber> + '_ {
        self.input_notes
            .values()
            .filter(|update| !update.inner().is_consumed())
            .filter_map(|update| {
                update.inner().inclusion_proof().map(|proof| proof.location().block_num())
            })
    }

    /// Returns whether the note is already tracked as an input or output record.
    pub fn tracks_note(&self, note_id: NoteId) -> bool {
        self.input_notes_by_id.contains_key(&note_id) || self.output_notes.contains_key(&note_id)
    }

    /// Returns whether an input note with the provided ID is tracked.
    pub fn has_input_note_with_id(&self, note_id: NoteId) -> bool {
        self.input_notes_by_id.contains_key(&note_id)
    }

    /// Returns whether an input note with the provided nullifier is tracked.
    pub fn has_input_note_with_nullifier(&self, nullifier: Nullifier) -> bool {
        self.input_notes_by_nullifier.contains_key(&nullifier)
    }

    /// Returns the input note record with the provided ID if it exists.
    pub fn input_note_by_id(&self, note_id: NoteId) -> Option<&InputNoteRecord> {
        let commitment = self.input_notes_by_id.get(&note_id)?;
        self.input_notes.get(commitment).map(InputNoteUpdate::inner)
    }

    /// Returns the input note record with the provided details commitment if it exists.
    pub fn input_note_by_commitment(
        &self,
        commitment: NoteDetailsCommitment,
    ) -> Option<&InputNoteRecord> {
        self.input_notes.get(&commitment).map(InputNoteUpdate::inner)
    }

    /// Returns the output note record with the provided ID if it exists.
    pub fn output_note_by_id(&self, note_id: NoteId) -> Option<&OutputNoteRecord> {
        self.output_notes.get(&note_id).map(OutputNoteUpdate::inner)
    }

    /// Returns the ID of the output note with the provided nullifier if it exists.
    pub fn output_note_id_by_nullifier(&self, nullifier: Nullifier) -> Option<NoteId> {
        self.output_notes_by_nullifier.get(&nullifier).copied()
    }

    /// Returns the position of the given nullifier in the consuming transaction order, or `None` if
    /// it is not present.
    pub fn get_nullifier_order(&self, nullifier: Nullifier) -> Option<u32> {
        self.nullifier_order.get(&nullifier).copied()
    }

    /// Returns the details commitment of a tracked metadata-less note whose id, combined with
    /// `metadata`, equals `note_id`, i.e. the committed note is that imported note.
    pub fn expected_note_matching(
        &self,
        note_id: NoteId,
        metadata: &NoteMetadata,
    ) -> Option<NoteDetailsCommitment> {
        self.input_notes
            .iter()
            .filter(|(_, update)| update.inner().metadata().is_none())
            .map(|(commitment, _)| *commitment)
            .find(|commitment| NoteId::new(*commitment, metadata) == note_id)
    }

    // UPDATE METHODS
    // --------------------------------------------------------------------------------------------

    /// Merges the note updates of a later transaction into this tracker. A note in both trackers
    /// takes the record from `later`. A note that this tracker inserts stays an insert, so the
    /// store writes the full record.
    pub fn merge(&mut self, later: NoteUpdateTracker) {
        for (commitment, update) in later.input_notes {
            let update_type = match self.input_notes.get(&commitment) {
                Some(earlier) if earlier.update_type == NoteUpdateType::Insert => {
                    NoteUpdateType::Insert
                },
                _ => update.update_type,
            };
            self.insert_input_note(update.note, update_type);
        }

        for update in later.output_notes.into_values() {
            self.insert_output_note(update.note, update.update_type);
        }
    }

    /// Refreshes the tracker with persisted input notes.
    ///
    /// Call this method before deriving state updates. Imported records can replace older records
    /// from the initial store snapshot. The records are already persisted, so they need no store
    /// update until their state changes.
    pub fn track_existing_input_notes(&mut self, notes: impl IntoIterator<Item = InputNoteRecord>) {
        for note in notes {
            self.insert_input_note(note, NoteUpdateType::None);
        }
    }

    /// Appends nullifiers to the per-account ordered nullifier list.
    ///
    /// Nullifiers from the same account must be in execution order; ordering across different
    /// accounts is not guaranteed.
    pub fn extend_nullifiers(&mut self, nullifiers: impl IntoIterator<Item = Nullifier>) {
        for nullifier in nullifiers {
            let next_pos =
                u32::try_from(self.nullifier_order.len()).expect("nullifier count exceeds u32");
            self.nullifier_order.entry(nullifier).or_insert(next_pos);
        }
    }

    /// Returns a mutable reference to the input note record with the provided ID if it exists.
    pub fn get_input_note_by_id(&mut self, note_id: NoteId) -> Option<&mut InputNoteRecord> {
        let commitment = self.input_notes_by_id.get(&note_id).copied()?;
        self.input_notes.get_mut(&commitment).map(InputNoteUpdate::inner_mut)
    }

    /// Returns a mutable reference to the output note record with the provided ID if it exists.
    pub fn get_output_note_by_id(&mut self, note_id: NoteId) -> Option<&mut OutputNoteRecord> {
        self.output_notes.get_mut(&note_id).map(OutputNoteUpdate::inner_mut)
    }

    /// Returns a mutable reference to the input note update with the provided nullifier if it
    /// exists.
    pub fn get_input_note_update_by_nullifier(
        &mut self,
        nullifier: Nullifier,
    ) -> Option<&mut InputNoteUpdate> {
        let commitment = self.input_notes_by_nullifier.get(&nullifier).copied()?;
        self.input_notes.get_mut(&commitment)
    }

    /// Returns a mutable reference to the output note record with the provided nullifier if it
    /// exists.
    pub fn get_output_note_by_nullifier(
        &mut self,
        nullifier: Nullifier,
    ) -> Option<&mut OutputNoteRecord> {
        let note_id = self.output_notes_by_nullifier.get(&nullifier).copied()?;
        self.output_notes.get_mut(&note_id).map(OutputNoteUpdate::inner_mut)
    }

    /// Insert an input note update
    pub fn insert_input_note(&mut self, note: InputNoteRecord, update_type: NoteUpdateType) {
        let update = match update_type {
            NoteUpdateType::None => InputNoteUpdate::new_none(note),
            NoteUpdateType::Insert => InputNoteUpdate::new_insert(note),
            NoteUpdateType::Update => InputNoteUpdate::new_update(note),
            NoteUpdateType::InsertCommitted => InputNoteUpdate::new_insert_committed(note),
        };

        let commitment = update.inner().details_commitment();
        if let Some(note_id) = update.inner().id() {
            // A note with metadata supersedes any metadata-less record for the same commitment.
            let nullifier = update.inner().nullifier().expect("note with an id has metadata");
            self.input_notes_by_nullifier.insert(nullifier, commitment);
            self.input_notes_by_id.insert(note_id, commitment);
            self.input_notes.insert(commitment, update);
        } else if self.input_notes.get(&commitment).is_none_or(|u| u.inner().id().is_none()) {
            // No metadata yet means no `NoteId` and no computable nullifier. Track by details
            // commitment until a committed note supplies the metadata to evolve it, but do not
            // overwrite a metadata-bearing record that already supersedes it.
            self.input_notes.insert(commitment, update);
        }
    }

    /// Releases notes that the discarded transaction was processing. Leaves all other notes
    /// unchanged.
    pub fn apply_transaction_discarded(
        &mut self,
        transaction_id: TransactionId,
    ) -> Result<(), NoteRecordError> {
        for update in self.input_notes.values_mut() {
            if update.note.transaction_discarded(transaction_id)? {
                // Mark the changed note for storage.
                update.inner_mut();
            }
        }

        Ok(())
    }

    /// Insert an output note update
    fn insert_output_note(&mut self, note: OutputNoteRecord, update_type: NoteUpdateType) {
        let note_id = note.id();
        if let Some(nullifier) = note.nullifier() {
            self.output_notes_by_nullifier.insert(nullifier, note_id);
        }
        let update = match update_type {
            NoteUpdateType::None => OutputNoteUpdate::new_none(note),
            NoteUpdateType::Update => OutputNoteUpdate::new_update(note),
            // Output notes are never assigned `InsertCommitted`; treat it as an insert for
            // exhaustiveness.
            NoteUpdateType::Insert | NoteUpdateType::InsertCommitted => {
                OutputNoteUpdate::new_insert(note)
            },
        };
        self.output_notes.insert(note_id, update);
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::vec;

    use miden_protocol::account::AccountId;
    use miden_protocol::block::BlockNumber;
    use miden_protocol::note::{
        NoteAssets,
        NoteAttachments,
        NoteDetails,
        NoteId,
        NoteMetadata,
        NoteRecipient,
        NoteStorage,
        NoteType,
        PartialNoteMetadata,
    };
    use miden_protocol::testing::account_id::ACCOUNT_ID_SENDER;
    use miden_protocol::transaction::TransactionId;
    use miden_protocol::{Felt, Word, ZERO};
    use miden_standards::note::StandardNote;

    use super::NoteUpdateTracker;
    use crate::store::InputNoteRecord;
    use crate::store::input_note_states::{
        ConsumedExternalNoteState,
        ConsumedUnauthenticatedLocalNoteState,
        NoteSubmissionData,
    };

    // HELPERS
    // --------------------------------------------------------------------------------------------

    fn note_details(seed: u64) -> NoteDetails {
        let serial_number: Word = [Felt::new_unchecked(seed), ZERO, ZERO, ZERO].into();
        let recipient = NoteRecipient::new(
            serial_number,
            StandardNote::SWAP.script(),
            NoteStorage::new(vec![]).unwrap(),
        );
        NoteDetails::new(NoteAssets::new(vec![]).unwrap(), recipient)
    }

    fn note_metadata(sender: AccountId) -> NoteMetadata {
        NoteMetadata::new(
            PartialNoteMetadata::new(sender, NoteType::Public),
            &NoteAttachments::empty(),
        )
    }

    /// A metadata-bearing note that is already consumed by a local transaction.
    fn consumed_local_note(seed: u64, sender: AccountId) -> InputNoteRecord {
        let state = ConsumedUnauthenticatedLocalNoteState {
            metadata: note_metadata(sender),
            nullifier_block_height: BlockNumber::from(1u32),
            submission_data: NoteSubmissionData {
                submitted_at: Some(0),
                consumer_account: sender,
                consumer_transaction: TransactionId::from_raw(Word::default()),
            },
            consumed_tx_order: Some(0),
        };
        InputNoteRecord::new(note_details(seed), NoteAttachments::empty(), Some(0), state.into())
    }

    /// A metadata-less note that is already externally consumed. It never carried a `NoteId`.
    fn consumed_external_note(seed: u64) -> InputNoteRecord {
        let state = ConsumedExternalNoteState {
            nullifier_block_height: BlockNumber::from(1u32),
            consumer_account: None,
            consumed_tx_order: None,
            metadata: None,
        };
        InputNoteRecord::new(note_details(seed), NoteAttachments::empty(), Some(0), state.into())
    }

    // TESTS
    // --------------------------------------------------------------------------------------------

    #[test]
    fn consumed_input_note_ids_reports_metadata_bearing_consumed_note() {
        let sender: AccountId = ACCOUNT_ID_SENDER.try_into().unwrap();
        let note = consumed_local_note(1, sender);
        let id = note.id().expect("consumed-local note has metadata");

        let tracker = NoteUpdateTracker::for_transaction_updates(vec![], vec![note], vec![]);

        let consumed: alloc::vec::Vec<NoteId> = tracker.consumed_input_note_ids().collect();
        assert_eq!(consumed, vec![id]);
    }

    #[test]
    fn consumed_input_note_ids_omits_note_that_never_had_an_id() {
        // A note inserted already in the externally-consumed (metadata-less) state never had an id
        // in the tracker, so it is persisted but is not reported by id.
        let note = consumed_external_note(2);
        assert!(note.id().is_none());

        let tracker = NoteUpdateTracker::for_transaction_updates(vec![note], vec![], vec![]);

        assert_eq!(tracker.consumed_input_note_ids().count(), 0);
        assert_eq!(tracker.updated_input_notes().count(), 1);
    }
}
