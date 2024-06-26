use crate::errors::{ConversionError, MissingFieldHelper};
use miden_objects::{
    notes::{NoteMetadata, NoteTag, NoteType},
    Felt,
};

impl TryFrom<crate::web_client::rpc::client_grpc::note::NoteMetadata> for NoteMetadata {
    type Error = ConversionError;

    fn try_from(
        value: crate::web_client::rpc::client_grpc::note::NoteMetadata,
    ) -> Result<Self, Self::Error> {
        let sender = value
            .sender
            .ok_or_else(|| {
                crate::web_client::rpc::client_grpc::note::NoteMetadata::missing_field("Sender")
            })?
            .try_into()?;
        let note_type = NoteType::try_from(value.note_type as u64)?;
        let tag = NoteTag::from(value.tag);
        let aux = Felt::try_from(value.aux).map_err(|_| ConversionError::NotAValidFelt)?;

        Ok(NoteMetadata::new(sender, note_type, tag, aux)?)
    }
}

impl From<NoteMetadata> for crate::web_client::rpc::client_grpc::note::NoteMetadata {
    fn from(val: NoteMetadata) -> Self {
        let sender = Some(val.sender().into());
        let note_type = val.note_type() as u32;
        let tag = val.tag().into();
        let aux = val.aux().into();

        crate::web_client::rpc::client_grpc::note::NoteMetadata { sender, note_type, tag, aux }
    }
}
