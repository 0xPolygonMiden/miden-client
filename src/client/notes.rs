use miden_objects::{
    assembly::ProgramAst,
    crypto::rand::FeltRng,
    notes::{NoteId, NoteInclusionProof, NoteScript},
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

    /// Imports a new input note into the client's store. The `verify` parameter dictates weather or
    /// not the method verifies the existence of the note in the chain.
    ///
    /// If the imported note is verified to be on chain and it doesn't contain an inclusion proof
    /// the method tries to build one if possible.
    /// If the verification fails then a [ClientError::ExistenceVerificationError] is raised.
    pub async fn import_input_note(
        &mut self,
        mut note: InputNoteRecord,
        verify: bool,
    ) -> Result<(), ClientError> {
        if !verify {
            return self.store.insert_input_note(&note).map_err(|err| err.into());
        }

        // Verify that note exists in chain
        let mut chain_notes = self.rpc_api.get_notes_by_id(&[note.id()]).await?;

        if chain_notes.is_empty() {
            return Err(ClientError::ExistenceVerificationError(note.id()));
        }

        let note_details = chain_notes.pop().expect("chain_notes should have at least one element");

        let inclusion_details = match note_details {
            super::rpc::NoteDetails::OffChain(_, _, inclusion) => inclusion,
            super::rpc::NoteDetails::Public(_, inclusion) => inclusion,
        };

        if note.inclusion_proof().is_none()
            && self.get_sync_height()? >= inclusion_details.block_num
        {
            // Add the inclusion proof to the imported note
            let block_header = self
                .rpc_api
                .get_block_header_by_number(Some(inclusion_details.block_num))
                .await?;

            let inclusion_proof = NoteInclusionProof::new(
                inclusion_details.block_num,
                block_header.sub_hash(),
                block_header.note_root(),
                inclusion_details.note_index.into(),
                inclusion_details.merkle_path,
            )?;

            note = InputNoteRecord::new(
                note.id(),
                note.recipient(),
                note.assets().clone(),
                note.status(),
                note.metadata().copied(),
                Some(inclusion_proof),
                note.details().clone(),
            );
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
