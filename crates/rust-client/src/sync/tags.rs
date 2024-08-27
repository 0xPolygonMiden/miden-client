use alloc::{collections::BTreeSet, vec::Vec};
use std::println;

use miden_objects::{
    crypto::rand::FeltRng,
    notes::{NoteExecutionMode, NoteTag},
};
use miden_tx::auth::TransactionAuthenticator;
use tracing::warn;
use winter_maybe_async::{maybe_async, maybe_await};

use crate::{
    errors::ClientError,
    rpc::NodeRpcClient,
    store::{NoteFilter, Store},
    Client,
};

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    /// Returns the list of note tags tracked by the client.
    ///
    /// When syncing the state with the node, these tags will be added to the sync request and
    /// note-related information will be retrieved for notes that have matching tags.
    ///
    /// Note: Tags for accounts that are being tracked by the client are managed automatically by
    /// the client and do not need to be added here. That is, notes for managed accounts will be
    /// retrieved automatically by the client when syncing.
    #[maybe_async]
    pub fn get_note_tags(&self) -> Result<Vec<NoteTag>, ClientError> {
        maybe_await!(self.store.get_note_tags()).map_err(|err| err.into())
    }

    /// Adds a note tag for the client to track.
    #[maybe_async]
    pub fn add_note_tag(&mut self, tag: NoteTag) -> Result<(), ClientError> {
        match maybe_await!(self.store.add_note_tag(tag)).map_err(|err| err.into()) {
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
        match maybe_await!(self.store.remove_note_tag(tag))? {
            true => Ok(()),
            false => {
                warn!("Tag {} wasn't being tracked", tag);
                Ok(())
            },
        }
    }

    /// Returns the list of note tags tracked by the client.
    #[maybe_async]
    pub(crate) fn get_tracked_note_tags(&self) -> Result<Vec<NoteTag>, ClientError> {
        let stored_tags = maybe_await!(self.get_note_tags())?;

        let account_tags = maybe_await!(self.get_account_stubs())?
            .into_iter()
            .map(|(stub, _)| NoteTag::from_account_id(stub.id(), NoteExecutionMode::Local))
            .collect::<Result<Vec<_>, _>>()?;

        let expected_notes = maybe_await!(self.store.get_input_notes(NoteFilter::Expected))?;

        let uncommited_note_tags: Vec<NoteTag> = expected_notes
            .iter()
            .filter_map(|note| note.metadata().map(|metadata| metadata.tag()))
            .collect();

        let imported_tags: Vec<NoteTag> =
            expected_notes.iter().filter_map(|note| note.imported_tag()).collect();

        let single_query = maybe_await!(self.store.testing_get_tracked_note_tag_single_query())?;

        println!("account_tags: {:?}", account_tags.clone());
        println!("stored_tags: {:?}", stored_tags.clone());
        println!("uncommited_note_tags: {:?}", uncommited_note_tags.clone());
        println!("imported_tags: {:?}", imported_tags.clone());

        println!("single_query: {:?}", single_query.clone());

        let multi_query = [account_tags, stored_tags, uncommited_note_tags, imported_tags]
            .concat()
            .into_iter()
            .collect::<BTreeSet<NoteTag>>()
            .into_iter()
            .collect();

        assert_eq!(single_query, multi_query);

        Ok(multi_query)
    }
}
