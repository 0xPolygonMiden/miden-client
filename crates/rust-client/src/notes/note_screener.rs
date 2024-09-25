use alloc::{collections::BTreeSet, rc::Rc, string::ToString, vec::Vec};
use core::fmt;

use miden_objects::{
    accounts::AccountId,
    assets::Asset,
    notes::{Note, NoteId},
    AccountError, AssetError, Word,
};
use winter_maybe_async::{maybe_async, maybe_await};

use super::script_roots::{P2ID, P2IDR, SWAP};
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
            NoteRelevance::After(height) => write!(f, "After block {}", height),
        }
    }
}

/// Provides functionality for testing whether a note is relevant to the client or not.
///
/// Here, relevance is based on whether the note is able to be consumed by an account that is
/// tracked in the provided `store`. This can be derived in a number of ways, such as looking
/// at the combination of script root and note inputs. For example, a P2ID note is relevant
/// for a specific account ID if this ID is its first note input.
pub struct NoteScreener<S: Store> {
    store: Rc<S>,
}

impl<S: Store> NoteScreener<S> {
    pub fn new(store: Rc<S>) -> Self {
        Self { store }
    }

    /// Returns a vector of tuples describing the relevance of the provided note to the
    /// accounts monitored by this screener.
    ///
    /// Does a fast check for known scripts (P2ID, P2IDR, SWAP). We're currently
    /// unable to execute notes that are not committed so a slow check for other scripts is
    /// currently not available.
    #[maybe_async]
    pub fn check_relevance(
        &self,
        note: &Note,
    ) -> Result<Vec<NoteConsumability>, NoteScreenerError> {
        let account_ids = BTreeSet::from_iter(maybe_await!(self.store.get_account_ids())?);

        let script_hash = note.script().hash().to_string();
        let note_relevance = match script_hash.as_str() {
            P2ID => Self::check_p2id_relevance(note, &account_ids)?,
            P2IDR => Self::check_p2idr_relevance(note, &account_ids)?,
            SWAP => maybe_await!(self.check_swap_relevance(note, &account_ids))?,
            _ => self.check_script_relevance(note, &account_ids)?,
        };

        Ok(note_relevance)
    }

    fn check_p2id_relevance(
        note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Result<Vec<NoteConsumability>, NoteScreenerError> {
        let mut note_inputs_iter = note.inputs().values().iter();
        let account_id_felt = note_inputs_iter
            .next()
            .ok_or(InvalidNoteInputsError::WrongNumInputs(note.id(), 1))?;

        if note_inputs_iter.next().is_some() {
            return Err(InvalidNoteInputsError::WrongNumInputs(note.id(), 1).into());
        }

        let account_id = AccountId::try_from(*account_id_felt)
            .map_err(|err| InvalidNoteInputsError::AccountError(note.id(), err))?;

        if !account_ids.contains(&account_id) {
            return Ok(vec![]);
        }
        Ok(vec![(account_id, NoteRelevance::Always)])
    }

    fn check_p2idr_relevance(
        note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Result<Vec<NoteConsumability>, NoteScreenerError> {
        let mut note_inputs_iter = note.inputs().values().iter();
        let account_id_felt = note_inputs_iter
            .next()
            .ok_or(InvalidNoteInputsError::WrongNumInputs(note.id(), 2))?;
        let recall_height_felt = note_inputs_iter
            .next()
            .ok_or(InvalidNoteInputsError::WrongNumInputs(note.id(), 2))?;

        if note_inputs_iter.next().is_some() {
            return Err(InvalidNoteInputsError::WrongNumInputs(note.id(), 2).into());
        }

        let sender = note.metadata().sender();
        let recall_height: u32 = recall_height_felt.as_int().try_into().map_err(|_err| {
            InvalidNoteInputsError::BlockNumberError(note.id(), recall_height_felt.as_int())
        })?;

        let account_id = AccountId::try_from(*account_id_felt)
            .map_err(|err| InvalidNoteInputsError::AccountError(note.id(), err))?;

        Ok(vec![
            (account_id, NoteRelevance::Always),
            (sender, NoteRelevance::After(recall_height)),
        ]
        .into_iter()
        .filter(|(account_id, _relevance)| account_ids.contains(account_id))
        .collect())
    }

    /// Checks if a swap note can be consumed by any account whose id is in `account_ids`
    ///
    /// This implementation serves as a placeholder as we're currently not able to create, execute
    /// and send SWAP NOTES. Hence, it's also untested. The main logic should be the same: for each
    /// account check if it has enough of the wanted asset.
    /// This is also very inefficient as we're loading the full accounts. We should instead just
    /// load the account's vaults, or even have a function in the `Store` to do this.
    ///
    /// TODO: test/revisit this in the future
    #[maybe_async]
    fn check_swap_relevance(
        &self,
        note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Result<Vec<NoteConsumability>, NoteScreenerError> {
        let note_inputs = note.inputs().values().to_vec();
        if note_inputs.len() != 9 {
            return Ok(Vec::new());
        }

        // get the demanded asset from the note's inputs
        let asset: Asset =
            Word::from([note_inputs[4], note_inputs[5], note_inputs[6], note_inputs[7]])
                .try_into()
                .map_err(|err| InvalidNoteInputsError::AssetError(note.id(), err))?;
        let asset_faucet_id = AccountId::try_from(asset.vault_key()[3])
            .map_err(|err| InvalidNoteInputsError::AccountError(note.id(), err))?;

        let mut accounts_with_relevance = Vec::new();

        for account_id in account_ids {
            let (account, _) = maybe_await!(self.store.get_account(*account_id))?;

            // Check that the account can cover the demanded asset
            match asset {
                Asset::NonFungible(_non_fungible_asset)
                    if account.vault().has_non_fungible_asset(asset).expect(
                        "Should be able to query has_non_fungible_asset for an Asset::NonFungible",
                    ) =>
                {
                    accounts_with_relevance.push((*account_id, NoteRelevance::Always))
                },
                Asset::Fungible(fungible_asset)
                    if account
                        .vault()
                        .get_balance(asset_faucet_id)
                        .expect("Should be able to query get_balance for an Asset::Fungible")
                        >= fungible_asset.amount() =>
                {
                    accounts_with_relevance.push((*account_id, NoteRelevance::Always))
                },
                _ => {},
            }
        }

        Ok(accounts_with_relevance)
    }

    fn check_script_relevance(
        &self,
        _note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Result<Vec<NoteConsumability>, NoteScreenerError> {
        // TODO: try to execute the note script against relevant accounts; this will
        // require querying data from the store
        Ok(account_ids
            .iter()
            .map(|account_id| (*account_id, NoteRelevance::Always))
            .collect())
    }
}

// NOTE SCREENER ERRORS
// ================================================================================================

/// Error when screening notes to check relevance to a client
#[derive(Debug)]
pub enum NoteScreenerError {
    InvalidNoteInputsError(InvalidNoteInputsError),
    StoreError(StoreError),
}

impl From<InvalidNoteInputsError> for NoteScreenerError {
    fn from(error: InvalidNoteInputsError) -> Self {
        Self::InvalidNoteInputsError(error)
    }
}

impl From<StoreError> for NoteScreenerError {
    fn from(error: StoreError) -> Self {
        Self::StoreError(error)
    }
}

impl fmt::Display for NoteScreenerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteScreenerError::InvalidNoteInputsError(note_inputs_err) => {
                write!(f, "error while processing note inputs: {note_inputs_err}")
            },
            NoteScreenerError::StoreError(store_error) => {
                write!(f, "error while fetching data from the store: {store_error}")
            },
        }
    }
}

#[derive(Debug)]
pub enum InvalidNoteInputsError {
    AccountError(NoteId, AccountError),
    AssetError(NoteId, AssetError),
    WrongNumInputs(NoteId, usize),
    BlockNumberError(NoteId, u64),
}

impl fmt::Display for InvalidNoteInputsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidNoteInputsError::AccountError(note_id, account_error) => {
                write!(f, "account error for note with ID {}: {account_error}", note_id.to_hex())
            },
            InvalidNoteInputsError::AssetError(note_id, asset_error) => {
                write!(f, "asset error for note with ID {}: {asset_error}", note_id.to_hex())
            },
            InvalidNoteInputsError::WrongNumInputs(note_id, expected_num_inputs) => {
                write!(
                    f,
                    "expected {expected_num_inputs} note inputs for note with ID {}",
                    note_id.to_hex()
                )
            },
            InvalidNoteInputsError::BlockNumberError(note_id, read_height) => {
                write!(
                    f,
                    "note input representing block with value {read_height} for note with ID {}",
                    note_id.to_hex()
                )
            },
        }
    }
}
