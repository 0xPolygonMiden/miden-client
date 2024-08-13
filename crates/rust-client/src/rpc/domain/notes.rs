use miden_objects::{
    notes::{NoteExecutionHint, NoteMetadata, NoteTag, NoteType},
    Felt,
};

use super::MissingFieldHelper;
#[cfg(feature = "tonic")]
use crate::rpc::tonic_client::generated::note::NoteMetadata as ProtoNoteMetadata;
#[cfg(feature = "web-tonic")]
use crate::rpc::web_tonic_client::generated::note::NoteMetadata as ProtoNoteMetadata;
use crate::rpc::RpcConversionError;

impl TryFrom<ProtoNoteMetadata> for NoteMetadata {
    type Error = RpcConversionError;

    fn try_from(value: ProtoNoteMetadata) -> Result<Self, Self::Error> {
        let sender = value
            .sender
            .ok_or_else(|| ProtoNoteMetadata::missing_field("Sender"))?
            .try_into()?;
        let note_type = NoteType::try_from(value.note_type as u64)?;
        let tag = NoteTag::from(value.tag);
        let execution_hint_tag = (value.execution_hint & 0xFF) as u8;
        let execution_hint_payload = ((value.execution_hint >> 8) & 0xFFFFFF) as u32;
        let execution_hint =
            NoteExecutionHint::from_parts(execution_hint_tag, execution_hint_payload)?;

        let aux = Felt::try_from(value.aux).map_err(|_| RpcConversionError::NotAValidFelt)?;

        Ok(NoteMetadata::new(sender, note_type, tag, execution_hint, aux)?)
    }
}
