use core::fmt;
use std::collections::BTreeSet;

use miden_objects::{accounts::AccountId, assets::Asset, notes::Note, Word};

use crate::{
    errors::{InvalidNoteInputsError, ScreenerError},
    store::Store,
};

// KNOWN SCRIPT ROOTS
// --------------------------------------------------------------------------------------------
pub(crate) const P2ID_NOTE_SCRIPT_ROOT: &str =
    "0xcdfd70344b952980272119bc02b837d14c07bbfc54f86a254422f39391b77b35";
pub(crate) const P2IDR_NOTE_SCRIPT_ROOT: &str =
    "0x41e5727b99a12b36066c09854d39d64dd09d9265c442a9be3626897572bf1745";
pub(crate) const SWAP_NOTE_SCRIPT_ROOT: &str =
    "0x5852920f88985b651cf7ef5e48623f898b6c292f4a2c25dd788ff8b46dd90417";

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteRelevance {
    /// The note can be consumed at any time.
    Always,
    /// The note can be consumed after the block with the specified number.
    After(u32),
}

impl fmt::Display for NoteRelevance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteRelevance::Always => write!(f, "Always"),
            NoteRelevance::After(height) => write!(f, "After block {}", height),
        }
    }
}

pub struct NoteScreener<'a, S: Store> {
    store: &'a S,
}

impl<'a, S: Store> NoteScreener<'a, S> {
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    /// Returns a vector of tuples describing the relevance of the provided note to the
    /// accounts monitored by this screener.
    ///
    /// Does a fast check for known scripts (P2ID, P2IDR, SWAP). We're currently
    /// unable to execute notes that are not committed so a slow check for other scripts is currently
    /// not available.
    pub fn check_relevance(
        &self,
        note: &Note,
    ) -> Result<Vec<(AccountId, NoteRelevance)>, ScreenerError> {
        let account_ids = BTreeSet::from_iter(self.store.get_account_ids()?);

        let script_hash = note.script().hash().to_string();
        let note_relevance = match script_hash.as_str() {
            P2ID_NOTE_SCRIPT_ROOT => Self::check_p2id_relevance(note, &account_ids)?,
            P2IDR_NOTE_SCRIPT_ROOT => Self::check_p2idr_relevance(note, &account_ids)?,
            SWAP_NOTE_SCRIPT_ROOT => self.check_swap_relevance(note, &account_ids)?,
            _ => self.check_script_relevance(note, &account_ids)?,
        };

        Ok(note_relevance)
    }

    fn check_p2id_relevance(
        note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Result<Vec<(AccountId, NoteRelevance)>, ScreenerError> {
        let mut note_inputs_iter = note.inputs().values().iter();
        let account_id_felt = note_inputs_iter
            .next()
            .ok_or(InvalidNoteInputsError::NumInputsError(note.id(), 1))?;

        if note_inputs_iter.next().is_some() {
            return Err(InvalidNoteInputsError::NumInputsError(note.id(), 1).into());
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
    ) -> Result<Vec<(AccountId, NoteRelevance)>, ScreenerError> {
        let mut note_inputs_iter = note.inputs().values().iter();
        let account_id_felt = note_inputs_iter
            .next()
            .ok_or(InvalidNoteInputsError::NumInputsError(note.id(), 2))?;
        let recall_height_felt = note_inputs_iter
            .next()
            .ok_or(InvalidNoteInputsError::NumInputsError(note.id(), 2))?;

        if note_inputs_iter.next().is_some() {
            return Err(InvalidNoteInputsError::NumInputsError(note.id(), 2).into());
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
    fn check_swap_relevance(
        &self,
        note: &Note,
        account_ids: &BTreeSet<AccountId>,
    ) -> Result<Vec<(AccountId, NoteRelevance)>, ScreenerError> {
        let note_inputs = note.inputs().to_vec();
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
            let (account, _) = self.store.get_account(*account_id)?;

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
    ) -> Result<Vec<(AccountId, NoteRelevance)>, ScreenerError> {
        // TODO: try to execute the note script against relevant accounts; this will
        // require querying data from the store
        Ok(account_ids
            .iter()
            .map(|account_id| (*account_id, NoteRelevance::Always))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use miden_lib::notes::{create_p2id_note, create_p2idr_note, create_swap_note};
    use miden_objects::{
        accounts::{AccountId, ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN},
        assets::FungibleAsset,
        crypto::rand::RpoRandomCoin,
        notes::NoteType,
    };

    use crate::client::note_screener::{
        P2IDR_NOTE_SCRIPT_ROOT, P2ID_NOTE_SCRIPT_ROOT, SWAP_NOTE_SCRIPT_ROOT,
    };

    // We need to make sure the script roots we use for filters are in line with the note scripts
    // coming from Miden objects
    #[test]
    fn ensure_correct_script_roots() {
        // create dummy data for the notes
        let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN.try_into().unwrap();
        let account_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN.try_into().unwrap();
        let rng = RpoRandomCoin::new(Default::default());

        // create dummy notes to compare note script roots
        let p2id_note = create_p2id_note(
            account_id,
            account_id,
            vec![FungibleAsset::new(faucet_id, 100u64).unwrap().into()],
            NoteType::OffChain,
            rng,
        )
        .unwrap();
        let p2idr_note = create_p2idr_note(
            account_id,
            account_id,
            vec![FungibleAsset::new(faucet_id, 100u64).unwrap().into()],
            NoteType::OffChain,
            10,
            rng,
        )
        .unwrap();
        let (swap_note, _serial_num) = create_swap_note(
            account_id,
            FungibleAsset::new(faucet_id, 100u64).unwrap().into(),
            FungibleAsset::new(faucet_id, 100u64).unwrap().into(),
            NoteType::OffChain,
            rng,
        )
        .unwrap();

        assert_eq!(p2id_note.script().hash().to_string(), P2ID_NOTE_SCRIPT_ROOT);
        assert_eq!(p2idr_note.script().hash().to_string(), P2IDR_NOTE_SCRIPT_ROOT);
        assert_eq!(swap_note.script().hash().to_string(), SWAP_NOTE_SCRIPT_ROOT);
    }
}
