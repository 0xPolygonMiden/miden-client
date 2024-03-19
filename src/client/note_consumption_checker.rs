use miden_objects::{
    accounts::AccountId,
    notes::{Note, NoteInputs},
};
use miden_tx::{DataStore, TransactionExecutor};

use crate::{errors::ClientError, store::Store};

// KNOWN SCRIPT ROOTS
// --------------------------------------------------------------------------------------------
pub(crate) const P2ID_NOTE_SCRIPT_ROOT: &str =
    "0x65c08aef0e3d11ce8a26662005a5272398e8810e5e13a903a993ee622d03675f";
pub(crate) const P2IDR_NOTE_SCRIPT_ROOT: &str =
    "0x03dd8f8fd57f015d821648292cee0ce42e16c4b80427c46b9cb874db44395f47";

/// Returns the indices of the notes from `created_notes` that can be consumed by the client
///
/// The provided `store` and `tx_executor` must correspond to the same client
pub fn filter_created_notes_to_track<S: Store, D: DataStore>(
    store: &mut S,
    tx_executor: &mut TransactionExecutor<D>,
    created_notes: &[Note],
) -> Result<Vec<usize>, ClientError> {
    let account_ids_tracked_by_client = store
        .get_account_stubs()?
        .iter()
        .map(|(account_stub, _seed)| account_stub.id())
        .collect::<Vec<_>>();

    let filtered_notes = created_notes
        .iter()
        .enumerate()
        .filter(|(_note_idx, note)| {
            is_note_relevant(tx_executor, note, &account_ids_tracked_by_client)
        })
        .map(|(note_idx, _note)| note_idx)
        .collect::<Vec<_>>();

    Ok(filtered_notes)
}

/// Returns whether the note is relevant to the client whose with transaction executor `tx_executor`
///
/// We call a note *irrelevant* if it cannot be consumed by any of the accounts corresponding to
/// `acount_id`. And a note is *relevant* if it's not *irrelevant* (this means it can for sure be
/// consumed or we can't be 100% sure it's possible to consume it)
fn is_note_relevant<D: DataStore>(
    tx_executor: &mut TransactionExecutor<D>,
    note: &Note,
    account_ids: &[AccountId],
) -> bool {
    account_ids
        .iter()
        .map(|&account_id| check_consumption(tx_executor, note, account_id))
        .any(|consumption_check_result| {
            consumption_check_result != NoteConsumptionCheckResult::NonConsumable
        })
}

/// Check if `note` can be consumed by the account corresponding to `account_id`
///
/// The function currently does a fast check for known scripts (P2ID and P2IDR). We're currently
/// unable to execute notes that are not committed so a slow check for other scripts is currently
/// not available.
pub fn check_consumption<D: DataStore>(
    _tx_executor: &mut TransactionExecutor<D>,
    note: &Note,
    account_id: AccountId,
) -> NoteConsumptionCheckResult {
    let script_hash_str = note.script().hash().to_string();
    let is_send_asset_note =
        script_hash_str == P2ID_NOTE_SCRIPT_ROOT || script_hash_str == P2IDR_NOTE_SCRIPT_ROOT;
    let note_inputs =
        NoteInputs::new(vec![(account_id).into()]).expect("Number of inputs should be 1");

    match is_send_asset_note {
        true if *note.inputs() == note_inputs => NoteConsumptionCheckResult::Consumable,
        true => NoteConsumptionCheckResult::NonConsumable,
        false => NoteConsumptionCheckResult::Unknown,
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteConsumptionCheckResult {
    Consumable,
    NonConsumable,
    Unknown,
}
