use alloc::{collections::BTreeSet, vec::Vec};
use core::fmt;

use miden_lib::note::well_known_note::WellKnownNote;
use miden_objects::{
    AccountError, AssetError, Felt, Word,
    account::{Account, AccountId},
    asset::Asset,
    note::{Note, NoteId},
};
use thiserror::Error;

use crate::store::{Store, StoreError};

/// Describes the relevance of a note based on the screening.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteRelevance {
    /// The note can be consumed at any time.
    Always,
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
            NoteRelevance::Always => write!(f, "Always"),
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
pub struct NoteScreener {
    store: alloc::sync::Arc<dyn Store>,
}

impl NoteScreener {
    pub fn new(store: alloc::sync::Arc<dyn Store>) -> Self {
        Self { store }
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
        let account_ids = BTreeSet::from_iter(self.store.get_account_ids().await?);

        let script_root = note.script().root();

        let note_relevance = if script_root == WellKnownNote::P2ID.script_root() {
            Self::check_p2id_relevance(note, &account_ids)?
        } else if script_root == WellKnownNote::P2IDR.script_root() {
            Self::check_p2idr_relevance(note, &account_ids)?
        } else if script_root == WellKnownNote::SWAP.script_root() {
            self.check_swap_relevance(note, &account_ids).await?
        } else {
            NoteScreener::check_script_relevance(note, &account_ids)
        };

        Ok(note_relevance)
    }

    fn check_p2id_relevance(
        note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Result<Vec<NoteConsumability>, NoteScreenerError> {
        let note_inputs = note.inputs().values();
        if note_inputs.len() != 2 {
            return Err(InvalidNoteInputsError::WrongNumInputs(note.id(), 2).into());
        }

        let account_id_felts: [Felt; 2] = note_inputs[0..2].try_into().expect(
            "Should be able to convert the first two note inputs to an array of two Felt elements",
        );

        let account_id =
            AccountId::try_from([account_id_felts[1], account_id_felts[0]]).map_err(|err| {
                InvalidNoteInputsError::AccountError(
                    note.id(),
                    AccountError::FinalAccountHeaderIdParsingFailed(err),
                )
            })?;

        if !account_ids.contains(&account_id) {
            return Ok(vec![]);
        }
        Ok(vec![(account_id, NoteRelevance::Always)])
    }

    fn check_p2idr_relevance(
        note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Result<Vec<NoteConsumability>, NoteScreenerError> {
        let note_inputs = note.inputs().values();
        if note_inputs.len() != 3 {
            return Err(InvalidNoteInputsError::WrongNumInputs(note.id(), 3).into());
        }

        let account_id_felts: [Felt; 2] = note_inputs[0..2].try_into().expect(
            "Should be able to convert the first two note inputs to an array of two Felt elements",
        );

        let recall_height_felt = note_inputs[2];

        let sender = note.metadata().sender();
        let recall_height: u32 = recall_height_felt.as_int().try_into().map_err(|_err| {
            InvalidNoteInputsError::BlockNumberError(note.id(), recall_height_felt.as_int())
        })?;

        let account_id =
            AccountId::try_from([account_id_felts[1], account_id_felts[0]]).map_err(|err| {
                InvalidNoteInputsError::AccountError(
                    note.id(),
                    AccountError::FinalAccountHeaderIdParsingFailed(err),
                )
            })?;

        Ok(vec![
            (account_id, NoteRelevance::Always),
            (sender, NoteRelevance::After(recall_height)),
        ]
        .into_iter()
        .filter(|(account_id, _relevance)| account_ids.contains(account_id))
        .collect())
    }

    /// Checks if a swap note can be consumed by any account whose ID is in `account_ids`.
    ///
    /// This implementation serves as a placeholder as we're currently not able to create, execute
    /// and send SWAP NOTES. Hence, it's also untested. The main logic should be the same: for each
    /// account check if it has enough of the wanted asset.
    /// This is also very inefficient as we're loading the full accounts. We should instead just
    /// load the account's vaults, or even have a function in the `Store` to do this.
    // TODO: test/revisit this in the future
    async fn check_swap_relevance(
        &self,
        note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Result<Vec<NoteConsumability>, NoteScreenerError> {
        let note_inputs = note.inputs().values();
        if note_inputs.len() != 10 {
            return Err(InvalidNoteInputsError::WrongNumInputs(note.id(), 10).into());
        }

        let asset_felts: [Felt; 4] = note_inputs[4..8].try_into().expect(
            "Should be able to convert the second word from note inputs to an array of four Felt elements",
        );

        // get the demanded asset from the note's inputs
        let asset: Asset = Word::from(asset_felts)
            .try_into()
            .map_err(|err| InvalidNoteInputsError::AssetError(note.id(), err))?;

        let mut accounts_with_relevance = Vec::new();

        for account_id in account_ids {
            let account: Account = self
                .store
                .get_account(*account_id)
                .await?
                .ok_or(NoteScreenerError::AccountDataNotFound(*account_id))?
                .into();

            // Check that the account can cover the demanded asset
            match asset {
                Asset::NonFungible(non_fungible_asset)
                    if account.vault().has_non_fungible_asset(non_fungible_asset).expect(
                        "Should be able to query has_non_fungible_asset for an Asset::NonFungible",
                    ) =>
                {
                    accounts_with_relevance.push((*account_id, NoteRelevance::Always));
                },
                Asset::Fungible(fungible_asset) => {
                    let asset_faucet_id = fungible_asset.faucet_id();
                    if account
                        .vault()
                        .get_balance(asset_faucet_id)
                        .expect("Should be able to query get_balance for an Asset::Fungible")
                        >= fungible_asset.amount()
                    {
                        accounts_with_relevance.push((*account_id, NoteRelevance::Always));
                    }
                },
                Asset::NonFungible(_) => {},
            }
        }

        Ok(accounts_with_relevance)
    }

    fn check_script_relevance(
        _note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Vec<NoteConsumability> {
        // TODO: try to execute the note script against relevant accounts; this will
        // require querying data from the store
        account_ids
            .iter()
            .map(|account_id| (*account_id, NoteRelevance::Always))
            .collect()
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
