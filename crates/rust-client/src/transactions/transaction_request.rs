use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use core::fmt;

use miden_objects::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    notes::{Note, NoteDetails, NoteId, NoteType},
    transaction::{TransactionArgs, TransactionScript},
    vm::AdviceMap,
    Word,
};

// MASM SCRIPTS
// --------------------------------------------------------------------------------------------

pub const AUTH_CONSUME_NOTES_SCRIPT: &str =
    include_str!("asm/transaction_scripts/auth_consume_notes.masm");
pub const DISTRIBUTE_FUNGIBLE_ASSET_SCRIPT: &str =
    include_str!("asm/transaction_scripts/distribute_fungible_asset.masm");
pub const AUTH_SEND_ASSET_SCRIPT: &str =
    include_str!("asm/transaction_scripts/auth_send_asset.masm");

// TRANSACTION REQUEST
// --------------------------------------------------------------------------------------------

pub type NoteArgs = Word;

/// Represents the most general way of defining an executable transaction
#[derive(Clone, Debug)]
pub struct TransactionRequest {
    /// ID of the account against which the transactions is to be executed.
    account_id: AccountId,
    // Notes to be consumed by the transaction that are not authenticated.
    unauthenticated_input_notes: Vec<Note>,
    /// Notes to be consumed by the transaction together with their (optional) arguments. This
    /// has to include both authenticated and unauthenticated notes.
    input_notes: BTreeMap<NoteId, Option<NoteArgs>>,
    /// A list of notes expected to be generated by the transactions.
    expected_output_notes: Vec<Note>,
    /// A list of note details of notes we expect to be created as part of future transactions.
    expected_partial_notes: Vec<NoteDetails>,
    /// Optional transaction script (together with its arguments).
    tx_script: Option<TransactionScript>,
    /// Initial state of the `AdviceMap` that provides data during runtime.
    advice_map: AdviceMap,
}

impl TransactionRequest {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    pub fn new(
        account_id: AccountId,
        unauthenticated_input_notes: Vec<Note>,
        input_notes: BTreeMap<NoteId, Option<NoteArgs>>,
        expected_output_notes: Vec<Note>,
        expected_partial_notes: Vec<NoteDetails>,
        tx_script: Option<TransactionScript>,
        advice_map: Option<AdviceMap>,
    ) -> Result<Self, TransactionRequestError> {
        if unauthenticated_input_notes
            .iter()
            .any(|note| !input_notes.contains_key(&note.id()))
        {
            return Err(TransactionRequestError::InputNotesMapMissingUnauthenticatedNotes);
        }

        Ok(Self {
            account_id,
            unauthenticated_input_notes,
            input_notes,
            expected_output_notes,
            expected_partial_notes,
            tx_script,
            advice_map: advice_map.unwrap_or_default(),
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    pub fn unauthenticated_input_notes(&self) -> &[Note] {
        &self.unauthenticated_input_notes
    }

    #[cfg(feature = "testing")]
    pub fn set_unauthenticated_input_notes(&mut self, unauthenticated_input_notes: Vec<Note>) {
        self.unauthenticated_input_notes = unauthenticated_input_notes;
    }

    pub fn unauthenticated_input_note_ids(&self) -> impl Iterator<Item = NoteId> + '_ {
        self.unauthenticated_input_notes.iter().map(|note| note.id())
    }

    pub fn authenticated_input_note_ids(&self) -> impl Iterator<Item = NoteId> + '_ {
        let unauthenticated_note_ids: BTreeSet<NoteId> =
            BTreeSet::from_iter(self.unauthenticated_input_note_ids());

        self.input_notes()
            .iter()
            .map(|(note_id, _)| *note_id)
            .filter(move |note_id| !unauthenticated_note_ids.contains(note_id))
    }

    pub fn input_notes(&self) -> &BTreeMap<NoteId, Option<NoteArgs>> {
        &self.input_notes
    }

    pub fn get_input_note_ids(&self) -> Vec<NoteId> {
        self.input_notes.keys().cloned().collect()
    }

    pub fn get_note_args(&self) -> BTreeMap<NoteId, NoteArgs> {
        self.input_notes
            .iter()
            .filter_map(|(note, args)| args.map(|a| (*note, a)))
            .collect()
    }

    pub fn expected_output_notes(&self) -> &[Note] {
        &self.expected_output_notes
    }

    pub fn expected_partial_notes(&self) -> &[NoteDetails] {
        &self.expected_partial_notes
    }

    pub fn tx_script(&self) -> Option<&TransactionScript> {
        self.tx_script.as_ref()
    }
}

impl From<TransactionRequest> for TransactionArgs {
    fn from(val: TransactionRequest) -> Self {
        let note_args = val.get_note_args();
        let mut tx_args = TransactionArgs::new(val.tx_script, Some(note_args), val.advice_map);

        let output_notes = val.expected_output_notes.into_iter();
        tx_args.extend_expected_output_notes(output_notes);

        tx_args
    }
}

#[derive(Debug)]
pub enum TransactionRequestError {
    InputNotesMapMissingUnauthenticatedNotes,
    InputNoteNotAuthenticated,
}
impl fmt::Display for TransactionRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotesMapMissingUnauthenticatedNotes => write!(f, "The input notes map should include keys for all provided unauthenticated input notes"),
            Self::InputNoteNotAuthenticated => write!(f, "Every authenticated note to be consumed should be committed and contain a valid inclusion proof"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionRequestError {}

// TRANSACTION TEMPLATE
// --------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum TransactionTemplate {
    /// Consume the specified notes against an account.
    ConsumeNotes(AccountId, Vec<NoteId>),
    /// Mint fungible assets using a faucet account and creates a note with the specified
    /// type that can be consumed by the target Account ID
    MintFungibleAsset(FungibleAsset, AccountId, NoteType),
    /// Creates a pay-to-id note with the specified type directed to a specific account
    PayToId(PaymentTransactionData, NoteType),
    /// Creates a pay-to-id note directed to a specific account, specifying a block height after
    /// which the note can be recalled
    PayToIdWithRecall(PaymentTransactionData, u32, NoteType),
    /// Creates a swap note offering a specific asset in exchange for another specific asset
    Swap(SwapTransactionData, NoteType),
}

impl TransactionTemplate {
    /// Returns the [AccountId] of the account which the transaction will be executed against
    pub fn account_id(&self) -> AccountId {
        match self {
            TransactionTemplate::ConsumeNotes(account_id, _) => *account_id,
            TransactionTemplate::MintFungibleAsset(asset, ..) => asset.faucet_id(),
            TransactionTemplate::PayToId(payment_data, _) => payment_data.account_id(),
            TransactionTemplate::PayToIdWithRecall(payment_data, ..) => payment_data.account_id(),
            TransactionTemplate::Swap(swap_data, ..) => swap_data.account_id(),
        }
    }
}

// PAYMENT TRANSACTION DATA
// --------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct PaymentTransactionData {
    asset: Asset,
    sender_account_id: AccountId,
    target_account_id: AccountId,
}

impl PaymentTransactionData {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    pub fn new(
        asset: Asset,
        sender_account_id: AccountId,
        target_account_id: AccountId,
    ) -> PaymentTransactionData {
        PaymentTransactionData {
            asset,
            sender_account_id,
            target_account_id,
        }
    }

    /// Returns the executor [AccountId]
    pub fn account_id(&self) -> AccountId {
        self.sender_account_id
    }

    /// Returns the target [AccountId]
    pub fn target_account_id(&self) -> AccountId {
        self.target_account_id
    }

    /// Returns the transaction [Asset]
    pub fn asset(&self) -> Asset {
        self.asset
    }
}

// SWAP TRANSACTION DATA
// --------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SwapTransactionData {
    sender_account_id: AccountId,
    offered_asset: Asset,
    requested_asset: Asset,
}

impl SwapTransactionData {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    pub fn new(
        sender_account_id: AccountId,
        offered_asset: Asset,
        requested_asset: Asset,
    ) -> SwapTransactionData {
        SwapTransactionData {
            sender_account_id,
            offered_asset,
            requested_asset,
        }
    }

    /// Returns the executor [AccountId]
    pub fn account_id(&self) -> AccountId {
        self.sender_account_id
    }

    /// Returns the transaction offered [Asset]
    pub fn offered_asset(&self) -> Asset {
        self.offered_asset
    }

    /// Returns the transaction requested [Asset]
    pub fn requested_asset(&self) -> Asset {
        self.requested_asset
    }
}

// KNOWN SCRIPT ROOTS
// --------------------------------------------------------------------------------------------

pub mod known_script_roots {
    pub const P2ID: &str = "0x3df15bd183c3239332dcb535c6d0a25c668ead19a317fefe66fc2754e49ce4f1";
    pub const P2IDR: &str = "0xf6513a4c607de61288263e1d9346889e9393f3c4024bfb42efc0e2ce3c64ee72";
    pub const SWAP: &str = "0x5040bdb39e3e71d8ae4a93d65ff44d152f56192df97018a63b6b6342e87f97d5";
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use miden_lib::notes::{create_p2id_note, create_p2idr_note, create_swap_note};
    use miden_objects::{
        accounts::{
            account_id::testing::{
                ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            },
            AccountId,
        },
        assets::FungibleAsset,
        crypto::rand::RpoRandomCoin,
        notes::NoteType,
        Felt, FieldElement,
    };

    use crate::transactions::known_script_roots::{P2ID, P2IDR, SWAP};

    // We need to make sure the script roots we use for filters are in line with the note scripts
    // coming from Miden objects
    #[test]
    fn ensure_correct_script_roots() {
        // create dummy data for the notes
        let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
        let account_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN.try_into().unwrap();
        let mut rng = RpoRandomCoin::new(Default::default());

        // create dummy notes to compare note script roots
        let p2id_note = create_p2id_note(
            account_id,
            account_id,
            vec![FungibleAsset::new(faucet_id, 100u64).unwrap().into()],
            NoteType::OffChain,
            Felt::ZERO,
            &mut rng,
        )
        .unwrap();
        let p2idr_note = create_p2idr_note(
            account_id,
            account_id,
            vec![FungibleAsset::new(faucet_id, 100u64).unwrap().into()],
            NoteType::OffChain,
            Felt::ZERO,
            10,
            &mut rng,
        )
        .unwrap();
        let (swap_note, _serial_num) = create_swap_note(
            account_id,
            FungibleAsset::new(faucet_id, 100u64).unwrap().into(),
            FungibleAsset::new(faucet_id, 100u64).unwrap().into(),
            NoteType::OffChain,
            Felt::ZERO,
            &mut rng,
        )
        .unwrap();

        assert_eq!(p2id_note.script().hash().to_string(), P2ID);
        assert_eq!(p2idr_note.script().hash().to_string(), P2IDR);
        assert_eq!(swap_note.script().hash().to_string(), SWAP);
    }
}
