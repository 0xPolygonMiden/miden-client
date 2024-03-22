use miden_objects::{accounts::AccountId, notes::Note};

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
pub fn filter_created_notes_to_track<S: Store>(
    store: &mut S,
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
        .filter(|(_note_idx, note)| is_note_relevant(note, &account_ids_tracked_by_client))
        .map(|(note_idx, _note)| note_idx)
        .collect::<Vec<_>>();

    Ok(filtered_notes)
}

/// Returns whether the note is relevant
///
/// We call a note *irrelevant* if it cannot be consumed by any of the accounts corresponding to
/// `acount_id`. And a note is *relevant* if it's not *irrelevant* (this means it can for sure be
/// consumed or we can't be 100% sure it's possible to consume it)
fn is_note_relevant(
    note: &Note,
    account_ids: &[AccountId],
) -> bool {
    account_ids
        .iter()
        .map(|&account_id| check_consumption(note, account_id))
        .any(|consumption_check_result| consumption_check_result != NoteRelevance::None)
}

/// Check if `note` can be consumed by the account corresponding to `account_id`
///
/// The function currently does a fast check for known scripts (P2ID and P2IDR). We're currently
/// unable to execute notes that are not committed so a slow check for other scripts is currently
/// not available.
pub fn check_consumption(
    note: &Note,
    account_id: AccountId,
) -> NoteRelevance {
    let script_hash_str = note.script().hash().to_string();
    let send_asset_inputs = vec![(account_id).into()];
    let note_inputs = note.inputs().to_vec();

    match script_hash_str.as_str() {
        P2ID_NOTE_SCRIPT_ROOT if note_inputs == send_asset_inputs => NoteRelevance::Always,
        P2IDR_NOTE_SCRIPT_ROOT
            if note_inputs.first() == send_asset_inputs.first() && note_inputs.len() > 1 =>
        {
            NoteRelevance::After(note_inputs[1].as_int() as u32)
        },
        P2ID_NOTE_SCRIPT_ROOT => NoteRelevance::None,
        P2IDR_NOTE_SCRIPT_ROOT => NoteRelevance::None,
        _ => NoteRelevance::Unknown,
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteRelevance {
    /// The note cannot be consumed.
    None,
    /// We cannot decide whether the note is consumable or not.
    Unknown,
    /// The note can be consumed at any time.
    Always,
    /// The note can be consumed after the block with the specified number.
    After(u32),
}
