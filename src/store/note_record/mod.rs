use miden_objects::{
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Word,
};
use serde::{Deserialize, Serialize};

mod input_note_record;
mod output_note_record;

pub use input_note_record::InputNoteRecord;
pub use output_note_record::OutputNoteRecord;

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
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_bytes(&[(*self).into()]);
    }
}

impl Deserializable for NoteStatus {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let enum_byte = u8::read_from(source)?;
        enum_byte.try_into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NoteRecordDetails {
    nullifier: String,
    script: Vec<u8>,
    inputs: Vec<u8>,
    serial_num: Word,
}

impl NoteRecordDetails {
    pub fn new(nullifier: String, script: Vec<u8>, inputs: Vec<u8>, serial_num: Word) -> Self {
        Self {
            nullifier,
            script,
            inputs,
            serial_num,
        }
    }

    pub fn script(&self) -> &Vec<u8> {
        &self.script
    }

    pub fn inputs(&self) -> &Vec<u8> {
        &self.inputs
    }

    pub fn serial_num(&self) -> Word {
        self.serial_num
    }
}

impl Serializable for NoteRecordDetails {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let nullifier_bytes = self.nullifier.as_bytes();
        target.write_usize(nullifier_bytes.len());
        target.write_bytes(nullifier_bytes);

        target.write_usize(self.script().len());
        target.write_bytes(self.script());

        target.write_usize(self.inputs().len());
        target.write_bytes(self.inputs());

        self.serial_num().write_into(target);
    }
}

impl Deserializable for NoteRecordDetails {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let nullifier_len = usize::read_from(source)?;
        let nullifier_bytes = source.read_vec(nullifier_len)?;
        let nullifier =
            String::from_utf8(nullifier_bytes).expect("Nullifier String bytes should be readable.");

        let script_len = usize::read_from(source)?;
        let script = source.read_vec(script_len)?;

        let inputs_len = usize::read_from(source)?;
        let inputs = source.read_vec(inputs_len)?;

        let serial_num = Word::read_from(source)?;

        Ok(NoteRecordDetails::new(
            nullifier, script, inputs, serial_num,
        ))
    }
}
