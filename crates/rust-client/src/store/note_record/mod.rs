//! This module defines common structs to be used within the [Store](crate::store::Store) for notes
//! that are available to be consumed ([InputNoteRecord]) and notes that have been produced as a
//! result of executing a transaction ([OutputNoteRecord]).
//!
//! # Features
//!
//! ## Serialization / Deserialization
//!
//! We provide serialization and deserialization support via [Serializable] and [Deserializable]
//! traits implementations, and also via [Serialize] and [Deserialize] from `serde`, to provide the
//! ability to serialize most fields into JSON. This is useful for example if you want to store
//! some fields as JSON columns like we do in
//! [SqliteStore](crate::store::sqlite_store::SqliteStore). For example, suppose we want to store
//! [InputNoteRecord]'s metadata field in a JSON column. In that case, we could do something like:
//!
//! ```ignore
//! fn insert_metadata_into_some_table(db: &mut Database, note: InputNoteRecord) {
//!     let note_metadata_json = serde_json::to_string(note.metadata()).unwrap();
//!
//!     db.execute("INSERT INTO notes_metadata (note_id, note_metadata) VALUES (?, ?)",
//!     note.id().to_hex(), note_metadata_json).unwrap()
//! }
//! ```
//!
//! ## Type conversion
//!
//! We also facilitate converting from/into [InputNote](miden_objects::transaction::InputNote) /
//! [Note](miden_objects::notes::Note), although this is not always possible. Check both
//! [InputNoteRecord]'s and [OutputNoteRecord]'s documentation for more details about this.

use alloc::string::{String, ToString};
use core::fmt;

use miden_objects::NoteError;

mod input_note_record;
mod output_note_record;

pub use input_note_record::{InputNoteRecord, InputNoteState};
pub use output_note_record::{NoteExportType, OutputNoteRecord, OutputNoteState};
pub mod input_note_states {
    pub use super::input_note_record::{
        CommittedNoteState, ConsumedAuthenticatedLocalNoteState, ExpectedNoteState,
        InvalidNoteState, ProcessingAuthenticatedNoteState, ProcessingUnauthenticatedNoteState,
    };
}

// NOTE RECORD ERROR
// ================================================================================================

/// Errors generated from note records.
#[derive(Debug)]
pub enum NoteRecordError {
    /// Error generated during conversion of note record.
    ConversionError(String),
    /// Invalid underlying note object.
    NoteError(NoteError),
    /// Note record is not consumable.
    NoteNotConsumable(String),
    /// Invalid inclusion proof.
    InvalidInclusionProof,
    /// Invalid state transition.
    InvalidStateTransition(String),
    /// Error generated during a state transition.
    StateTransitionError(String),
}

impl fmt::Display for NoteRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use NoteRecordError::*;
        match self {
            ConversionError(msg) => write!(f, "Note record conversion error: {}", msg),
            NoteError(err) => write!(f, "Note error: {}", err),
            NoteNotConsumable(msg) => write!(f, "Note not consumable: {}", msg),
            InvalidInclusionProof => write!(f, "Invalid inclusion proof"),
            InvalidStateTransition(msg) => write!(f, "Invalid state transition: {}", msg),
            StateTransitionError(msg) => write!(f, "State transition error: {}", msg),
        }
    }
}

impl From<NoteError> for NoteRecordError {
    fn from(error: NoteError) -> Self {
        NoteRecordError::NoteError(error)
    }
}

impl From<NoteRecordError> for String {
    fn from(err: NoteRecordError) -> String {
        err.to_string()
    }
}
