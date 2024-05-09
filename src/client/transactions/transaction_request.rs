use alloc::collections::BTreeMap;

use miden_objects::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    notes::{Note, NoteId, NoteType},
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
    /// Notes to be consumed by the transaction together with their (optional) arguments.
    input_notes: BTreeMap<NoteId, Option<NoteArgs>>,
    /// A list of notes expected to be generated by the transactions.
    expected_output_notes: Vec<Note>,
    /// Optional transaction script (together with its arguments).
    tx_script: Option<TransactionScript>,
}

impl TransactionRequest {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    pub fn new(
        account_id: AccountId,
        input_notes: BTreeMap<NoteId, Option<NoteArgs>>,
        expected_output_notes: Vec<Note>,
        tx_script: Option<TransactionScript>,
    ) -> Self {
        Self {
            account_id,
            input_notes,
            expected_output_notes,
            tx_script,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub fn account_id(&self) -> AccountId {
        self.account_id
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

    pub fn tx_script(&self) -> Option<&TransactionScript> {
        self.tx_script.as_ref()
    }
}

impl From<TransactionRequest> for TransactionArgs {
    fn from(val: TransactionRequest) -> Self {
        let note_args = val.get_note_args();
        let mut tx_args = TransactionArgs::new(val.tx_script, Some(note_args), AdviceMap::new());

        let output_notes = val.expected_output_notes.into_iter();
        tx_args.extend_expected_output_notes(output_notes);

        tx_args
    }
}

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
}

impl TransactionTemplate {
    /// Returns the [AccountId] of the account which the transaction will be executed against
    pub fn account_id(&self) -> AccountId {
        match self {
            TransactionTemplate::ConsumeNotes(account_id, _) => *account_id,
            TransactionTemplate::MintFungibleAsset(asset, ..) => asset.faucet_id(),
            TransactionTemplate::PayToId(payment_data, _) => payment_data.account_id(),
            TransactionTemplate::PayToIdWithRecall(payment_data, ..) => payment_data.account_id(),
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

// KNOWN SCRIPT HASHES
// --------------------------------------------------------------------------------------------
pub struct KnownScriptHash;

impl KnownScriptHash {
    pub const P2ID: &'static str =
        "0xcdfd70344b952980272119bc02b837d14c07bbfc54f86a254422f39391b77b35";
    pub const P2IDR: &'static str =
        "0x41e5727b99a12b36066c09854d39d64dd09d9265c442a9be3626897572bf1745";
    pub const SWAP: &'static str =
        "0x5852920f88985b651cf7ef5e48623f898b6c292f4a2c25dd788ff8b46dd90417";
}
