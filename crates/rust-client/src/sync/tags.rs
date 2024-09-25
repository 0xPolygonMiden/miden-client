use alloc::{collections::BTreeSet, vec::Vec};

use miden_objects::{
    accounts::AccountId,
    crypto::rand::FeltRng,
    notes::{NoteExecutionMode, NoteId, NoteTag},
};
use miden_tx::{
    auth::TransactionAuthenticator,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
};
use tracing::warn;
use winter_maybe_async::{maybe_async, maybe_await};

use crate::{errors::ClientError, store::NoteFilter, Client};

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
        if !maybe_await!(self
            .store
            .remove_note_tag(NoteTagRecord { tag, source: NoteTagSource::User }))?
        {
            warn!("Tag {} wasn't being tracked", tag);
        }

        Ok(())
    }

    /// Returns the list of note tags tracked by the client.
    #[maybe_async]
    pub(crate) fn get_tracked_note_tags(&self) -> Result<Vec<NoteTag>, ClientError> {
        let stored_tags = maybe_await!(self.get_note_tags())?.into_iter().map(|r| r.tag).collect();

        let account_tags = maybe_await!(self.get_account_headers())?
            .into_iter()
            .map(|(header, _)| NoteTag::from_account_id(header.id(), NoteExecutionMode::Local))
            .collect::<Result<Vec<_>, _>>()?;

        let expected_notes = maybe_await!(self.store.get_input_notes(NoteFilter::Expected))?;

        let uncommited_note_tags: Vec<NoteTag> = expected_notes
            .iter()
            .filter_map(|note| note.metadata().map(|metadata| metadata.tag()))
            .collect();

        Ok([account_tags, stored_tags, uncommited_note_tags]
            .concat()
            .into_iter()
            .collect::<BTreeSet<NoteTag>>()
            .into_iter()
            .collect())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NoteTagRecord {
    pub tag: NoteTag,
    pub source: NoteTagSource,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NoteTagSource {
    Account(AccountId),
    Note(NoteId),
    User,
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
