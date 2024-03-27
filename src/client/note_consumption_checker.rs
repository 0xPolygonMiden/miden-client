use miden_objects::{accounts::AccountId, notes::Note};

use crate::{errors::ClientError, store::Store};

// KNOWN SCRIPT ROOTS
// --------------------------------------------------------------------------------------------
pub(crate) const P2ID_NOTE_SCRIPT_ROOT: &str =
    "0x65c08aef0e3d11ce8a26662005a5272398e8810e5e13a903a993ee622d03675f";
pub(crate) const P2IDR_NOTE_SCRIPT_ROOT: &str =
    "0x03dd8f8fd57f015d821648292cee0ce42e16c4b80427c46b9cb874db44395f47";
pub(crate) const SWAP_NOTE_SCRIPT_ROOT: &str =
    "0x0270336bdc66b9cfd0b7988f56b2e3e1cb39c920ec37627e49390523280c1545";

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
            if (note.metadata().sender() == account_id
                || note_inputs.first() == send_asset_inputs.first())
                && note_inputs.len() == 2 =>
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

#[cfg(test)]
mod tests {
    use miden_lib::{
        notes::{create_p2id_note, create_p2idr_note, create_swap_note},
        AuthScheme,
    };
    use miden_objects::{
        accounts::{AccountId, AccountType},
        assets::FungibleAsset,
        crypto::{dsa::rpo_falcon512::KeyPair, rand::RpoRandomCoin},
        Felt,
    };
    use rand::Rng;

    use crate::client::note_consumption_checker::{
        P2IDR_NOTE_SCRIPT_ROOT, P2ID_NOTE_SCRIPT_ROOT, SWAP_NOTE_SCRIPT_ROOT,
    };

    // We need to make sure the script roots we use for filters are in line with the note scripts
    // coming from Miden objects
    #[test]
    fn ensure_correct_script_roots() {
        // create dummy data for the notes
        let faucet_id: AccountId = 10347894387879516201u64.try_into().unwrap();

        let key_pair: KeyPair = KeyPair::new().unwrap();
        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
            pub_key: key_pair.public_key(),
        };

        // we need to use an initial seed to create the wallet account
        let mut rng = rand::thread_rng();
        let init_seed: [u8; 32] = rng.gen();

        let (account, _seed) = miden_lib::accounts::wallets::create_basic_wallet(
            init_seed,
            auth_scheme,
            AccountType::RegularAccountImmutableCode,
        )
        .unwrap();
        let account_id = account.id();

        let rng = {
            let coin_seed: [u64; 4] = rng.gen();
            RpoRandomCoin::new(coin_seed.map(Felt::new))
        };

        // create dummy notes to compare note script roots
        let p2id_note = create_p2id_note(
            account_id,
            account_id,
            vec![FungibleAsset::new(faucet_id, 100u64).unwrap().into()],
            rng,
        )
        .unwrap();
        let p2idr_note = create_p2idr_note(
            account_id,
            account_id,
            vec![FungibleAsset::new(faucet_id, 100u64).unwrap().into()],
            10,
            rng,
        )
        .unwrap();
        let (swap_note, _serial_num) = create_swap_note(
            account_id,
            miden_objects::assets::Asset::Fungible(
                FungibleAsset::new(faucet_id, 100u64).unwrap().into(),
            ),
            miden_objects::assets::Asset::Fungible(
                FungibleAsset::new(faucet_id, 100u64).unwrap().into(),
            ),
            rng,
        )
        .unwrap();

        assert_eq!(p2id_note.script().hash().to_string(), P2ID_NOTE_SCRIPT_ROOT);
        assert_eq!(p2idr_note.script().hash().to_string(), P2IDR_NOTE_SCRIPT_ROOT);
        assert_eq!(swap_note.script().hash().to_string(), SWAP_NOTE_SCRIPT_ROOT);
    }
}
