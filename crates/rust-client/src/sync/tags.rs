use alloc::{string::ToString, vec::Vec};

use miden_objects::{
    accounts::{Account, AccountId},
    crypto::rand::FeltRng,
    notes::{NoteExecutionMode, NoteId, NoteTag},
    NoteError,
};
use miden_tx::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};
use tracing::warn;

use crate::{
    errors::ClientError,
    store::{InputNoteRecord, NoteRecordError},
    Client,
};

/// Tag management methods
impl<R: FeltRng> Client<R> {
    /// Returns the list of note tags tracked by the client along with their source.
    ///
    /// When syncing the state with the node, these tags will be added to the sync request and
    /// note-related information will be retrieved for notes that have matching tags.
    ///  The source of the tag indicates its origin. It helps distinguish between:
    ///  - Tags added manually by the user.
    ///  - Tags automatically added by the client to track notes.
    ///  - Tags added for accounts tracked by the client.
    ///
    /// Note: Tags for accounts that are being tracked by the client are managed automatically by
    /// the client and don't need to be added here. That is, notes for managed accounts will be
    /// retrieved automatically by the client when syncing.
    pub async fn get_note_tags(&self) -> Result<Vec<NoteTagRecord>, ClientError> {
        self.store.get_note_tags().await.map_err(|err| err.into())
    }

    /// Adds a note tag for the client to track. This tag's source will be marked as `User`.
    pub async fn add_note_tag(&mut self, tag: NoteTag) -> Result<(), ClientError> {
        match self
            .store
            .add_note_tag(NoteTagRecord { tag, source: NoteTagSource::User })
            .await
            .map_err(|err| err.into())
        {
            Ok(true) => Ok(()),
            Ok(false) => {
                warn!("Tag {} is already being tracked", tag);
                Ok(())
            },
            Err(err) => Err(err),
        }
    }

    /// Removes a note tag for the client to track. Only tags added by the user can be removed.
    pub async fn remove_note_tag(&mut self, tag: NoteTag) -> Result<(), ClientError> {
        if self
            .store
            .remove_note_tag(NoteTagRecord { tag, source: NoteTagSource::User })
            .await?
            == 0
        {
            warn!("Tag {} wasn't being tracked", tag);
        }

        Ok(())
    }
}

/// Represents a note tag of which the Store can keep track and retrieve.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NoteTagRecord {
    pub tag: NoteTag,
    pub source: NoteTagSource,
}

/// Represents the source of the tag. This is used to differentiate between tags that are added by
/// the user and tags that are added automatically by the client to track notes .
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NoteTagSource {
    /// Tag for notes directed to a tracked account.
    Account(AccountId),
    /// Tag for tracked expected notes.
    Note(NoteId),
    /// Tag manually added by the user.
    User,
}

impl NoteTagRecord {
    pub fn with_note_source(tag: NoteTag, note_id: NoteId) -> Self {
        Self {
            tag,
            source: NoteTagSource::Note(note_id),
        }
    }

    pub fn with_account_source(tag: NoteTag, account_id: AccountId) -> Self {
        Self {
            tag,
            source: NoteTagSource::Account(account_id),
        }
    }
}

impl Serializable for NoteTagSource {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            NoteTagSource::Account(account_id) => {
                target.write_u8(0);
                account_id.write_into(target);
            },
            NoteTagSource::Note(note_id) => {
                target.write_u8(1);
                note_id.write_into(target);
            },
            NoteTagSource::User => target.write_u8(2),
        }
    }
}

impl Deserializable for NoteTagSource {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        match source.read_u8()? {
            0 => Ok(NoteTagSource::Account(AccountId::read_from(source)?)),
            1 => Ok(NoteTagSource::Note(NoteId::read_from(source)?)),
            2 => Ok(NoteTagSource::User),
            val => Err(DeserializationError::InvalidValue(format!("Invalid tag source: {}", val))),
        }
    }
}

impl PartialEq<NoteTag> for NoteTagRecord {
    fn eq(&self, other: &NoteTag) -> bool {
        self.tag == *other
    }
}

impl TryInto<NoteTagRecord> for &InputNoteRecord {
    type Error = NoteRecordError;

    fn try_into(self) -> Result<NoteTagRecord, Self::Error> {
        match self.metadata() {
            Some(metadata) => Ok(NoteTagRecord::with_note_source(metadata.tag(), self.id())),
            None => Err(NoteRecordError::ConversionError(
                "Input Note Record does not contain tag".to_string(),
            )),
        }
    }
}

impl TryInto<NoteTagRecord> for &Account {
    type Error = NoteError;
    fn try_into(self) -> Result<NoteTagRecord, Self::Error> {
        Ok(NoteTagRecord::with_account_source(
            NoteTag::from_account_id(self.id(), NoteExecutionMode::Local)?,
            self.id(),
        ))
    }
}
