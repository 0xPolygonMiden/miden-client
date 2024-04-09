use miden_objects::{
    assembly::{Assembler, ProgramAst},
    notes::NoteScript,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest, Felt, Word,
};
use serde::{Deserialize, Serialize};

mod input_note_record;
mod output_note_record;

pub use input_note_record::InputNoteRecord;
pub use output_note_record::OutputNoteRecord;

/// This module defines common structs to be used within the [Store](crate::store::Store) for notes
/// that are available to be consumed ([InputNoteRecord]) and notes that have been produced as a
/// result of executing a transaction ([OutputNoteRecord]).
///
/// # Features
///
/// ## Serialization / Deserialization
///
/// We provide serialization and deserialization support via [Serializable] and [Deserializable]
/// traits implementations, and also via [Serialize] and [Deserialize] from `serde` to provide the
/// ability to serialize most fields into JSON. This is useful for example if you want to store
/// some fields as json columns like we do in
/// [SqliteStore](crate::store::sqlite_store::SqliteStore). For example, suppose we want to store
/// [InputNoteRecord]'s metadata field in a JSON column. In that case, we could do something like:
///
/// ```ignore
/// fn insert_metadata_into_some_table(db: &mut Database, note: InputNoteRecord) {
///     let note_metadata_json = serde_json::to_string(note.metadata()).unwrap();
///
///     db.execute("INSERT INTO notes_metadata (note_id, note_metadata) VALUES (?, ?)",
///     note.id().to_hex(), note_metadata_json).unwrap()
/// }
/// ```
///
/// ## Type conversion
///
/// We also facilitate converting from/into [InputNote](miden_objects::transaction::InputNote) /
/// [Note](miden_objects::notes::Note), although this is not always possible. Check both
/// [InputNoteRecord]'s and [OutputNoteRecord]'s documentation for more details into this

// NOTE STATUS
// ================================================================================================
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NoteStatus {
    Pending,
    Committed,
    Consumed,
}

impl From<NoteStatus> for u8 {
    fn from(value: NoteStatus) -> Self {
        match value {
            NoteStatus::Pending => 0,
            NoteStatus::Committed => 1,
            NoteStatus::Consumed => 2,
        }
    }
}

impl TryFrom<u8> for NoteStatus {
    type Error = DeserializationError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(NoteStatus::Pending),
            1 => Ok(NoteStatus::Committed),
            2 => Ok(NoteStatus::Consumed),
            _ => Err(DeserializationError::InvalidValue(value.to_string())),
        }
    }
}

impl Serializable for NoteStatus {
    fn write_into<W: ByteWriter>(
        &self,
        target: &mut W,
    ) {
        target.write_bytes(&[(*self).into()]);
    }
}

impl Deserializable for NoteStatus {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let enum_byte = u8::read_from(source)?;
        enum_byte.try_into()
    }
}

fn default_script() -> NoteScript {
    let assembler = Assembler::default();
    let note_program_ast =
        ProgramAst::parse("begin end").expect("dummy script should be parseable");
    let (note_script, _) = NoteScript::new(note_program_ast, &assembler)
        .expect("dummy note script should be created without issues");
    note_script
}

// NOTE: NoteInputs does not impl Serialize which is why we use Vec<Felt> here
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NoteRecordDetails {
    nullifier: String,
    script_hash: Digest,
    #[serde(skip_serializing, skip_deserializing, default = "default_script")]
    script: NoteScript,
    inputs: Vec<Felt>,
    serial_num: Word,
}

impl NoteRecordDetails {
    pub fn new(
        nullifier: String,
        script: NoteScript,
        inputs: Vec<Felt>,
        serial_num: Word,
    ) -> Self {
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
    fn write_into<W: ByteWriter>(
        &self,
        target: &mut W,
    ) {
        let nullifier_bytes = self.nullifier.as_bytes();
        target.write_usize(nullifier_bytes.len());
        target.write_bytes(nullifier_bytes);

        self.script().write_into(target);

        target.write_usize(self.inputs.len());
        target.write_many(self.inputs());

        self.serial_num().write_into(target);
    }
}

impl Deserializable for NoteRecordDetails {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let nullifier_len = usize::read_from(source)?;
        let nullifier_bytes = source.read_vec(nullifier_len)?;
        let nullifier =
            String::from_utf8(nullifier_bytes).expect("Nullifier String bytes should be readable.");

        let script = NoteScript::read_from(source)?;

        let inputs_len = source.read_usize()?;
        let inputs = source.read_many::<Felt>(inputs_len)?;

        let serial_num = Word::read_from(source)?;

        Ok(NoteRecordDetails::new(nullifier, script, inputs, serial_num))
    }
}
