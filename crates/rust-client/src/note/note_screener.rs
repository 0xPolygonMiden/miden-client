use alloc::{sync::Arc, vec::Vec};
use core::fmt;

use miden_lib::{account::interface::AccountInterface, note::well_known_note::WellKnownNote};
use miden_objects::{
    AccountError, AssetError,
    account::{Account, AccountId},
    note::{Note, NoteId},
    transaction::{InputNote, InputNotes},
};
use miden_tx::{
    NoteAccountExecution, NoteConsumptionChecker, TransactionExecutor, TransactionExecutorError,
    TransactionMastStore,
};
use thiserror::Error;

use crate::{
    store::{Store, StoreError},
    transaction::{TransactionRequestBuilder, TransactionRequestError},
};

/// Describes the relevance of a note based on the screening.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteRelevance {
    /// The note can be consumed in the current block.
    Now,
    /// The note can be consumed after the block with the specified number.
    After(u32),
}

/// Represents the consumability of a note by a specific account.
///
/// The tuple contains the account ID that may consume the note and the moment it will become
/// relevant.
pub type NoteConsumability = (AccountId, NoteRelevance);

impl fmt::Display for NoteRelevance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteRelevance::Now => write!(f, "Now"),
            NoteRelevance::After(height) => write!(f, "After block {height}"),
        }
    }
}

/// Provides functionality for testing whether a note is relevant to the client or not.
///
/// Here, relevance is based on whether the note is able to be consumed by an account that is
/// tracked in the provided `store`. This can be derived in a number of ways, such as looking
/// at the combination of script root and note inputs. For example, a P2ID note is relevant
/// for a specific account ID if this ID is its first note input.
pub struct NoteScreener<'a> {
    /// A reference to the client's store, used to fetch necessary data to check consumability.
    store: Arc<dyn Store>,
    /// A consumability checker, used to check whether a note can be consumed by an account.
    consumability_checker: NoteConsumptionChecker<'a>,
    /// A MAST store, used to provide code inputs to the VM.
    mast_store: Arc<TransactionMastStore>,
}

impl<'a> NoteScreener<'a> {
    pub fn new(
        store: Arc<dyn Store>,
        tx_executor: &'a TransactionExecutor,
        mast_store: Arc<TransactionMastStore>,
    ) -> Self {
        Self {
            store,
            consumability_checker: NoteConsumptionChecker::new(tx_executor),
            mast_store,
        }
    }

    /// Returns a vector of tuples describing the relevance of the provided note to the
    /// accounts monitored by this screener.
    ///
    /// Does a fast check for known scripts (P2ID, P2IDR, SWAP). We're currently
    /// unable to execute notes that aren't committed so a slow check for other scripts is
    /// currently not available.
    pub async fn check_relevance(
        &self,
        note: &Note,
    ) -> Result<Vec<NoteConsumability>, NoteScreenerError> {
        let mut note_relevances = vec![];
        for id in self.store.get_account_ids().await? {
            let account_record = self
                .store
                .get_account(id)
                .await?
                .ok_or(NoteScreenerError::AccountDataNotFound(id))?;

            if let Some(relevance) =
                self.check_standard_consumability(account_record.account(), note).await?
            {
                note_relevances.push((id, relevance));
            } else {
                // The note might be consumable after a certain block height if the note is p2idr
                let script_root = note.script().root();

                if script_root == WellKnownNote::P2IDR.script_root() {
                    if let Some(relevance) = Self::check_p2idr_recall_consumability(note, &id)? {
                        note_relevances.push((id, relevance));
                    }
                }
            }
        }

        Ok(note_relevances)
    }

    /// Tries to execute a standard consume transaction to check if the note is consumable by the
    /// account.
    async fn check_standard_consumability(
        &self,
        account: &Account,
        note: &Note,
    ) -> Result<Option<NoteRelevance>, NoteScreenerError> {
        let transaction_request =
            TransactionRequestBuilder::consume_notes(vec![note.id()]).build()?;

        let tx_script =
            transaction_request.build_transaction_script(&AccountInterface::from(account), true)?;

        let tx_args = transaction_request.clone().into_transaction_args(tx_script, vec![]);
        let input_notes = InputNotes::new(vec![InputNote::unauthenticated(note.clone())])
            .expect("Single note should be valid");

        self.mast_store.load_transaction_code(account.code(), &input_notes, &tx_args);

        if let NoteAccountExecution::Success = self
            .consumability_checker
            .check_notes_consumability(
                account.id(),
                self.store.get_sync_height().await?,
                input_notes,
                tx_args,
            )
            .await?
        {
            return Ok(Some(NoteRelevance::Now));
        }

        Ok(None)
    }

    /// Special relevance check for P2IDR notes. It checks if the sender account can consume and
    /// recall the note.
    fn check_p2idr_recall_consumability(
        note: &Note,
        account_id: &AccountId,
    ) -> Result<Option<NoteRelevance>, NoteScreenerError> {
        let note_inputs = note.inputs().values();
        if note_inputs.len() != 3 {
            return Err(InvalidNoteInputsError::WrongNumInputs(note.id(), 3).into());
        }

        let recall_height_felt = note_inputs[2];

        let sender = note.metadata().sender();
        let recall_height: u32 = recall_height_felt.as_int().try_into().map_err(|_err| {
            InvalidNoteInputsError::BlockNumberError(note.id(), recall_height_felt.as_int())
        })?;

        if sender == *account_id {
            Ok(Some(NoteRelevance::After(recall_height)))
        } else {
            Ok(None)
        }
    }
}

// NOTE SCREENER ERRORS
// ================================================================================================

/// Error when screening notes to check relevance to a client.
#[derive(Debug, Error)]
pub enum NoteScreenerError {
    #[error("error while processing note inputs")]
    InvalidNoteInputsError(#[from] InvalidNoteInputsError),
    #[error("account data wasn't found for account id {0}")]
    AccountDataNotFound(AccountId),
    #[error("error while fetching data from the store")]
    StoreError(#[from] StoreError),
    #[error("error while checking consume transaction")]
    TransactionExecutionError(#[from] TransactionExecutorError),
    #[error("error while building consume transaction request")]
    TransactionRequestError(#[from] TransactionRequestError),
}

#[derive(Debug, Error)]
pub enum InvalidNoteInputsError {
    #[error("account error for note with id {0}: {1}")]
    AccountError(NoteId, AccountError),
    #[error("asset error for note with id {0}: {1}")]
    AssetError(NoteId, AssetError),
    #[error("expected {1} note inputs for note with id {0}")]
    WrongNumInputs(NoteId, usize),
    #[error("note input representing block with value {1} for note with id {0}")]
    BlockNumberError(NoteId, u64),
}
