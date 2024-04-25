use miden_objects::{
    assembly::ProgramAst,
    crypto::rand::FeltRng,
    notes::{NoteId, NoteScript},
};
use miden_tx::ScriptTarget;

use super::{rpc::NodeRpcClient, Client};
use crate::{
    errors::ClientError,
    store::{InputNoteRecord, NoteFilter, Store},
};

impl<N: NodeRpcClient, R: FeltRng, S: Store> Client<N, R, S> {
    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_input_notes(&self, filter: NoteFilter) -> Result<Vec<InputNoteRecord>, ClientError> {
        self.store.get_input_notes(filter).map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(&self, note_id: NoteId) -> Result<InputNoteRecord, ClientError> {
        self.store.get_input_note(note_id).map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Imports a new input note into the client's store.
    pub async fn import_input_note(
        &mut self,
        note: InputNoteRecord,
        verify: bool,
    ) -> Result<(), ClientError> {
        if verify {
            let mut chain_notes = self.rpc_api.get_notes_by_id(&[note.id()]).await?;

            if chain_notes.is_empty() {
                return Err(ClientError::ExistanceVerificationError(note.id()));
            }

            let note_details =
                chain_notes.pop().expect("chain_notes should have at least one element");

            let inclusion_details = match note_details {
                super::rpc::NoteDetails::OffChain(_, _, inclusion) => inclusion,
                super::rpc::NoteDetails::Public(_, inclusion) => inclusion,
            };

            if self.get_sync_height()? > inclusion_details.block_num {
                //Set inclusion proof
            }
        }
        self.store.insert_input_note(&note).map_err(|err| err.into())
    }

    /// Compiles the provided program into a [NoteScript] and checks (to the extent possible) if
    /// the specified note program could be executed against all accounts with the specified
    /// interfaces.
    pub fn compile_note_script(
        &self,
        note_script_ast: ProgramAst,
        target_account_procs: Vec<ScriptTarget>,
    ) -> Result<NoteScript, ClientError> {
        self.tx_executor
            .compile_note_script(note_script_ast, target_account_procs)
            .map_err(ClientError::TransactionExecutorError)
    }
}
