use miden_objects::{
    notes::{NoteMetadata, NoteTag, NoteType},
    Felt,
};

use super::MissingFieldHelper;
use crate::{
    errors::RpcConversionError, rpc::tonic_client::generated::note::NoteMetadata as NoteMetadataPb,
};

impl TryFrom<NoteMetadataPb> for NoteMetadata {
    type Error = RpcConversionError;

    fn try_from(value: NoteMetadataPb) -> Result<Self, Self::Error> {
        let sender = value
            .sender
            .ok_or_else(|| NoteMetadataPb::missing_field("Sender"))?
            .try_into()?;
        let note_type = NoteType::try_from(value.note_type as u64)?;
        let tag = NoteTag::from(value.tag);
        let aux = Felt::try_from(value.aux).map_err(|_| RpcConversionError::NotAValidFelt)?;

        Ok(NoteMetadata::new(sender, note_type, tag, aux)?)
    }
}
