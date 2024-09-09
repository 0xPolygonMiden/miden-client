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

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Display};

use chrono::{Local, TimeZone};
use miden_objects::{
    accounts::AccountId,
    notes::{Note, NoteDetails, NoteInclusionProof, NoteMetadata, NoteScript, NoteTag},
    transaction::TransactionId,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest, Felt, Word,
};

mod input_note_record;
mod output_note_record;

pub use input_note_record::InputNoteRecord;
pub use output_note_record::OutputNoteRecord;

// NOTE STATUS
// ================================================================================================
pub const NOTE_STATUS_EXPECTED: &str = "Expected";
pub const NOTE_STATUS_COMMITTED: &str = "Committed";
pub const NOTE_STATUS_CONSUMED: &str = "Consumed";
pub const NOTE_STATUS_PROCESSING: &str = "Processing";

/// Possible status for a single note. They describe the note's state and dictate its lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteStatus {
    /// Note is expected to be committed on chain.
    Expected {
        /// UNIX epoch-based timestamp (in seconds) when the note (either new or imported) started
        /// being tracked by the client. If the timestamp is not known, this field will be `None`.
        created_at: Option<u64>,
        /// Block height at which the note is expected to be committed. If the block height is not
        /// known, this field will be `None`.
        block_height: Option<u32>,
    },
    /// Note has been committed on chain, and can be consumed in a transaction.
    Committed {
        /// Block height at which the note was committed.
        block_height: u32,
    },
    /// Note has been consumed locally but not yet nullified on chain.
    Processing {
        /// ID of account that is consuming the note.
        consumer_account_id: AccountId,
        /// UNIX epoch-based timestamp (in seconds) of the note's consumption.
        submitted_at: u64,
    },
    /// Note has been nullified on chain.
    Consumed {
        /// ID of account that consumed the note. If the consumer account is not known, this field
        /// will be `None`.
        consumer_account_id: Option<AccountId>,
        /// Block height at which the note was consumed.
        block_height: u32,
    },
}

impl Serializable for NoteStatus {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            NoteStatus::Expected { created_at, block_height } => {
                target.write_u8(0);
                created_at.write_into(target);
                block_height.write_into(target);
            },
            NoteStatus::Committed { block_height } => {
                target.write_u8(1);
                block_height.write_into(target);
            },
            NoteStatus::Processing { consumer_account_id, submitted_at } => {
                target.write_u8(2);
                submitted_at.write_into(target);
                consumer_account_id.write_into(target);
            },
            NoteStatus::Consumed { consumer_account_id, block_height } => {
                target.write_u8(3);
                block_height.write_into(target);
                consumer_account_id.write_into(target);
            },
        }
    }
}

impl Deserializable for NoteStatus {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let status = source.read_u8()?;
        match status {
            0 => {
                let created_at = Option::<u64>::read_from(source)?;
                let block_height = Option::<u32>::read_from(source)?;
                Ok(NoteStatus::Expected { created_at, block_height })
            },
            1 => {
                let block_height = source.read_u32()?;
                Ok(NoteStatus::Committed { block_height })
            },
            2 => {
                let submitted_at = source.read_u64()?;
                let consumer_account_id = AccountId::read_from(source)?;
                Ok(NoteStatus::Processing { consumer_account_id, submitted_at })
            },
            3 => {
                let block_height = source.read_u32()?;
                let consumer_account_id = Option::<AccountId>::read_from(source)?;
                Ok(NoteStatus::Consumed { consumer_account_id, block_height })
            },
            _ => Err(DeserializationError::InvalidValue("NoteStatus".to_string())),
        }
    }
}

impl Display for NoteStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteStatus::Expected { created_at, block_height } => write!(
                f,
                "{NOTE_STATUS_EXPECTED} (created at {} and expected after block height {})",
                created_at
                    .map(|ts| Local
                        .timestamp_opt(ts as i64, 0)
                        .single()
                        .expect("timestamp should be valid")
                        .to_string())
                    .unwrap_or("?".to_string()),
                block_height.map(|h| h.to_string()).unwrap_or("?".to_string())
            ),
            NoteStatus::Committed { block_height } => {
                write!(f, "{NOTE_STATUS_COMMITTED} (at block height {block_height})")
            },
            NoteStatus::Processing { consumer_account_id, submitted_at } => write!(
                f,
                "{NOTE_STATUS_PROCESSING} (submitted at {} by account {})",
                Local
                    .timestamp_opt(*submitted_at as i64, 0)
                    .single()
                    .expect("timestamp should be valid"),
                consumer_account_id.to_hex()
            ),
            NoteStatus::Consumed { consumer_account_id, block_height } => write!(
                f,
                "{NOTE_STATUS_CONSUMED} (at block height {block_height} by account {})",
                consumer_account_id.map(|id| id.to_hex()).unwrap_or("?".to_string())
            ),
        }
    }
}

// NOTE: NoteInputs does not impl Serialize which is why we use Vec<Felt> here
#[derive(Clone, Debug, PartialEq)]
pub struct NoteRecordDetails {
    nullifier: String,
    script_hash: Digest,
    script: NoteScript,
    inputs: Vec<Felt>,
    serial_num: Word,
}

impl NoteRecordDetails {
    pub fn new(nullifier: String, script: NoteScript, inputs: Vec<Felt>, serial_num: Word) -> Self {
        let script_hash = script.hash();
        Self {
            nullifier,
            script,
            script_hash,
            inputs,
            serial_num,
        }
    }

    pub fn nullifier(&self) -> &str {
        &self.nullifier
    }

    pub fn script_hash(&self) -> &Digest {
        &self.script_hash
    }

    pub fn script(&self) -> &NoteScript {
        &self.script
    }

    pub fn inputs(&self) -> &Vec<Felt> {
        &self.inputs
    }

    pub fn serial_num(&self) -> Word {
        self.serial_num
    }
}

impl Serializable for NoteRecordDetails {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let nullifier_bytes = self.nullifier.as_bytes();
        target.write_u64(nullifier_bytes.len() as u64);
        target.write_bytes(nullifier_bytes);

        self.script().write_into(target);

        target.write_u64(self.inputs.len() as u64);
        target.write_many(self.inputs());

        self.serial_num().write_into(target);
    }
}

impl Deserializable for NoteRecordDetails {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let nullifier_len = u64::read_from(source)? as usize;
        let nullifier_bytes = source.read_vec(nullifier_len)?;
        let nullifier =
            String::from_utf8(nullifier_bytes).expect("Nullifier String bytes should be readable.");

        let script = NoteScript::read_from(source)?;

        let inputs_len = source.read_u64()? as usize;
        let inputs = source.read_many::<Felt>(inputs_len)?;

        let serial_num = Word::read_from(source)?;

        Ok(NoteRecordDetails::new(nullifier, script, inputs, serial_num))
    }
}

impl From<Note> for NoteRecordDetails {
    fn from(note: Note) -> Self {
        Self::new(
            note.nullifier().to_string(),
            note.script().clone(),
            note.inputs().values().to_vec(),
            note.serial_num(),
        )
    }
}

impl From<NoteDetails> for NoteRecordDetails {
    fn from(details: NoteDetails) -> Self {
        Self::new(
            details.nullifier().to_string(),
            details.script().clone(),
            details.inputs().values().to_vec(),
            details.serial_num(),
        )
    }
}

// NOTE STATE
// ================================================================================================

pub const STATE_UNKNOWN: u8 = 0;
pub const STATE_EXPECTED: u8 = 1;
pub const STATE_UNVERIFIED: u8 = 2;
pub const STATE_COMMITTED: u8 = 3;
pub const STATE_INVALID: u8 = 4;
pub const STATE_PROCESSING_AUTHENTICATED: u8 = 5;
pub const STATE_PROCESSING_UNAUTHENTICATED: u8 = 6;
pub const STATE_NATIVE_CONSUMED: u8 = 7;
pub const STATE_FOREIGN_CONSUMED: u8 = 8;

#[derive(Clone, Debug, PartialEq)]
pub enum NoteState {
    Unknown,
    Expected {
        after_block_num: u32,
        tag: NoteTag,
    },
    Unverified {
        metadata: NoteMetadata,
        inclusion_proof: NoteInclusionProof,
    },
    Committed {
        metadata: NoteMetadata,
        inclusion_proof: NoteInclusionProof,
        block_note_root: Digest,
    },
    Invalid {
        metadata: NoteMetadata,
        invalid_inclusion_proof: NoteInclusionProof,
        block_note_root: Digest,
    },
    ProcessingAuthenticated {
        metadata: NoteMetadata,
        inclusion_proof: NoteInclusionProof,
        block_note_root: Digest,
        submission_data: NoteSubmissionData,
    },
    ProcessingUnauthenticated {
        after_block_num: u32,
        submission_data: NoteSubmissionData,
    },
    NativeConsumed {
        metadata: NoteMetadata,
        inclusion_proof: NoteInclusionProof,
        block_note_root: Digest,
        nullifier_block_height: u32,
        submission_data: NoteSubmissionData,
    },
    ForeignConsumed {
        nullifier_block_height: u32,
    },
}

impl NoteState {
    pub fn discriminant(&self) -> u8 {
        match self {
            NoteState::Unknown => STATE_UNKNOWN,
            NoteState::Expected { .. } => STATE_EXPECTED,
            NoteState::Unverified { .. } => STATE_UNVERIFIED,
            NoteState::Committed { .. } => STATE_COMMITTED,
            NoteState::Invalid { .. } => STATE_INVALID,
            NoteState::ProcessingAuthenticated { .. } => STATE_PROCESSING_AUTHENTICATED,
            NoteState::ProcessingUnauthenticated { .. } => STATE_PROCESSING_UNAUTHENTICATED,
            NoteState::NativeConsumed { .. } => STATE_NATIVE_CONSUMED,
            NoteState::ForeignConsumed { .. } => STATE_FOREIGN_CONSUMED,
        }
    }
}

impl Serializable for NoteState {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u8(self.discriminant());
        match self {
            NoteState::Unknown => (),
            NoteState::Expected { after_block_num, tag } => {
                after_block_num.write_into(target);
                tag.write_into(target);
            },
            NoteState::Unverified { metadata, inclusion_proof } => {
                metadata.write_into(target);
                inclusion_proof.write_into(target);
            },
            NoteState::Committed {
                metadata,
                inclusion_proof,
                block_note_root,
            } => {
                metadata.write_into(target);
                inclusion_proof.write_into(target);
                block_note_root.write_into(target);
            },
            NoteState::Invalid {
                metadata,
                invalid_inclusion_proof,
                block_note_root,
            } => {
                metadata.write_into(target);
                invalid_inclusion_proof.write_into(target);
                block_note_root.write_into(target);
            },
            NoteState::ProcessingAuthenticated {
                metadata,
                inclusion_proof,
                block_note_root,
                submission_data,
            } => {
                metadata.write_into(target);
                inclusion_proof.write_into(target);
                block_note_root.write_into(target);
                submission_data.write_into(target);
            },
            NoteState::ProcessingUnauthenticated { after_block_num, submission_data } => {
                after_block_num.write_into(target);
                submission_data.write_into(target);
            },
            NoteState::NativeConsumed {
                metadata,
                inclusion_proof,
                block_note_root,
                nullifier_block_height,
                submission_data,
            } => {
                metadata.write_into(target);
                inclusion_proof.write_into(target);
                block_note_root.write_into(target);
                nullifier_block_height.write_into(target);
                submission_data.write_into(target);
            },
            NoteState::ForeignConsumed { nullifier_block_height } => {
                nullifier_block_height.write_into(target);
            },
        }
    }
}

impl Display for NoteState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteState::Unknown => write!(f, "Unknown"),
            NoteState::Expected { after_block_num, .. } => {
                write!(f, "Expected (after block {})", after_block_num)
            },
            NoteState::Unverified { inclusion_proof, .. } => {
                write!(
                    f,
                    "Unverified (with commit block {})",
                    inclusion_proof.location().block_num()
                )
            },
            NoteState::Committed { inclusion_proof, .. } => {
                write!(f, "Committed (at block height {})", inclusion_proof.location().block_num())
            },
            NoteState::Invalid { invalid_inclusion_proof, .. } => {
                write!(
                    f,
                    "Invalid (with commit block {})",
                    invalid_inclusion_proof.location().block_num()
                )
            },
            NoteState::ProcessingAuthenticated { submission_data, .. } => {
                write!(
                    f,
                    "Processing Authenticated (submitted at {} by account {})",
                    submission_data
                        .submitted_at
                        .map(|submitted_at| {
                            Local
                                .timestamp_opt(submitted_at as i64, 0)
                                .single()
                                .expect("timestamp should be valid")
                                .to_string()
                        })
                        .unwrap_or("?".to_string()),
                    submission_data.consumer_account
                )
            },
            NoteState::ProcessingUnauthenticated { submission_data, .. } => {
                write!(
                    f,
                    "Processing Unauthenticated (submitted at {} by account {})",
                    submission_data
                        .submitted_at
                        .map(|submitted_at| {
                            Local
                                .timestamp_opt(submitted_at as i64, 0)
                                .single()
                                .expect("timestamp should be valid")
                                .to_string()
                        })
                        .unwrap_or("?".to_string()),
                    submission_data.consumer_account
                )
            },
            NoteState::NativeConsumed {
                nullifier_block_height, submission_data, ..
            } => {
                write!(
                    f,
                    "Natively Consumed (at block {} by account {})",
                    nullifier_block_height, submission_data.consumer_account
                )
            },
            NoteState::ForeignConsumed { nullifier_block_height } => {
                write!(f, "Foreignly Consumed (at block {})", nullifier_block_height)
            },
        }
    }
}

impl Deserializable for NoteState {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let discriminant = source.read_u8()?;
        match discriminant {
            STATE_UNKNOWN => Ok(NoteState::Unknown),
            STATE_EXPECTED => {
                let after_block_num = u32::read_from(source)?;
                let tag = NoteTag::read_from(source)?;
                Ok(NoteState::Expected { after_block_num, tag })
            },
            STATE_UNVERIFIED => {
                let metadata = NoteMetadata::read_from(source)?;
                let inclusion_proof = NoteInclusionProof::read_from(source)?;
                Ok(NoteState::Unverified { metadata, inclusion_proof })
            },
            STATE_COMMITTED => {
                let metadata = NoteMetadata::read_from(source)?;
                let inclusion_proof = NoteInclusionProof::read_from(source)?;
                let block_note_root = Digest::read_from(source)?;
                Ok(NoteState::Committed {
                    metadata,
                    inclusion_proof,
                    block_note_root,
                })
            },
            STATE_INVALID => {
                let metadata = NoteMetadata::read_from(source)?;
                let invalid_inclusion_proof = NoteInclusionProof::read_from(source)?;
                let block_note_root = Digest::read_from(source)?;
                Ok(NoteState::Invalid {
                    metadata,
                    invalid_inclusion_proof,
                    block_note_root,
                })
            },
            STATE_PROCESSING_AUTHENTICATED => {
                let metadata = NoteMetadata::read_from(source)?;
                let inclusion_proof = NoteInclusionProof::read_from(source)?;
                let block_note_root = Digest::read_from(source)?;
                let submission_data = NoteSubmissionData::read_from(source)?;
                Ok(NoteState::ProcessingAuthenticated {
                    metadata,
                    inclusion_proof,
                    block_note_root,
                    submission_data,
                })
            },
            STATE_PROCESSING_UNAUTHENTICATED => {
                let after_block_num = u32::read_from(source)?;
                let submission_data = NoteSubmissionData::read_from(source)?;
                Ok(NoteState::ProcessingUnauthenticated { after_block_num, submission_data })
            },
            STATE_NATIVE_CONSUMED => {
                let metadata = NoteMetadata::read_from(source)?;
                let inclusion_proof = NoteInclusionProof::read_from(source)?;
                let block_note_root = Digest::read_from(source)?;
                let nullifier_block_height = u32::read_from(source)?;
                let submission_data = NoteSubmissionData::read_from(source)?;
                Ok(NoteState::NativeConsumed {
                    metadata,
                    inclusion_proof,
                    block_note_root,
                    nullifier_block_height,
                    submission_data,
                })
            },
            STATE_FOREIGN_CONSUMED => {
                let nullifier_block_height = u32::read_from(source)?;
                Ok(NoteState::ForeignConsumed { nullifier_block_height })
            },
            _ => Err(DeserializationError::InvalidValue(format!(
                "Invalid NoteState discriminant: {}",
                discriminant
            ))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoteSubmissionData {
    pub submitted_at: u64,
    pub consumer_account: AccountId,
    pub consumer_transaction: TransactionId,
}

impl Serializable for NoteSubmissionData {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.submitted_at.write_into(target);
        self.consumer_account.write_into(target);
        self.consumer_transaction.write_into(target);
    }
}

impl Deserializable for NoteSubmissionData {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let submitted_at = u64::read_from(source)?;
        let consumer_account = AccountId::read_from(source)?;
        let consumer_transaction = TransactionId::read_from(source)?;
        Ok(NoteSubmissionData {
            submitted_at,
            consumer_account,
            consumer_transaction,
        })
    }
}
