use miden_objects::{
    notes::{NoteMetadata, NoteTag, NoteType},
    Felt,
};

use super::MissingFieldHelper;
use crate::rpc::errors::RpcConversionError;
#[cfg(feature = "tonic")]
use crate::rpc::tonic_client::generated::note::NoteMetadata as ProtoNoteMetadata;
#[cfg(feature = "web-tonic")]
use crate::rpc::web_tonic_client::generated::note::NoteMetadata as ProtoNoteMetadata;

impl TryFrom<ProtoNoteMetadata> for NoteMetadata {
    type Error = RpcConversionError;

    fn try_from(value: ProtoNoteMetadata) -> Result<Self, Self::Error> {
        let sender = value
            .sender
            .ok_or_else(|| ProtoNoteMetadata::missing_field("Sender"))?
            .try_into()?;
        let note_type = NoteType::try_from(value.note_type as u64)?;
        let tag = NoteTag::from(value.tag);
        let aux = Felt::try_from(value.aux).map_err(|_| RpcConversionError::NotAValidFelt)?;

        Ok(NoteMetadata::new(sender, note_type, tag, aux)?)
    }
}
