use alloc::{collections::BTreeSet, string::ToString, vec::Vec};

use miden_objects::{
    accounts::{Account, AccountId},
    crypto::rand::FeltRng,
    notes::{NoteExecutionMode, NoteId, NoteTag},
    NoteError,
};
use miden_tx::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};
use tracing::warn;
use winter_maybe_async::{maybe_async, maybe_await};

use crate::{
    errors::ClientError,
    store::{InputNoteRecord, NoteRecordError},
    Client,
};

impl<R: FeltRng> Client<R> {
    /// Returns the list of note tags tracked by the client.
    ///
    /// When syncing the state with the node, these tags will be added to the sync request and
    /// note-related information will be retrieved for notes that have matching tags.
    ///
    /// Note: Tags for accounts that are being tracked by the client are managed automatically by
    /// the client and do not need to be added here. That is, notes for managed accounts will be
    /// retrieved automatically by the client when syncing.
    #[maybe_async]
    pub fn get_note_tags(&self) -> Result<Vec<NoteTagRecord>, ClientError> {
        maybe_await!(self.store.get_note_tags()).map_err(|err| err.into())
    }

    /// Returns the unique note tags (without source) that the client is interested in.
    #[maybe_async]
    pub fn get_unique_note_tags(&self) -> Result<BTreeSet<NoteTag>, ClientError> {
        maybe_await!(self.store.get_unique_note_tags()).map_err(|err| err.into())
    }

    /// Adds a note tag for the client to track.
    #[maybe_async]
    pub fn add_note_tag(&mut self, tag: NoteTag) -> Result<(), ClientError> {
        match maybe_await!(self
            .store
            .add_note_tag(NoteTagRecord { tag, source: NoteTagSource::User }))
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

    /// Removes a note tag for the client to track.
    #[maybe_async]
    pub fn remove_note_tag(&mut self, tag: NoteTag) -> Result<(), ClientError> {
        if maybe_await!(self
            .store
            .remove_note_tag(NoteTagRecord { tag, source: NoteTagSource::User }))?
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
