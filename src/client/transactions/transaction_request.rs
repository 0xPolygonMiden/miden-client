use miden_objects::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    notes::{Note, NoteId},
    transaction::{TransactionArgs, TransactionScript},
    utils::collections::BTreeMap,
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
pub struct TransactionRequest {
    /// ID of the account against which the transactions is to be executed.
    account_id: AccountId,
    /// Notes to be consumed by the transaction together with their arguments.
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

    pub fn get_note_ids(&self) -> Vec<NoteId> {
        self.input_notes.keys().cloned().collect()
    }

    pub fn get_note_args(&self) -> BTreeMap<NoteId, NoteArgs> {
        self.input_notes
            .iter()
            .filter(|(_, args)| args.is_some())
            .map(|(note, args)| (*note, args.expect("safe to unwrap due to filter")))
            .collect()
    }

    pub fn expected_output_notes(&self) -> &Vec<Note> {
        &self.expected_output_notes
    }

    pub fn tx_script(&self) -> Option<&TransactionScript> {
        self.tx_script.as_ref()
    }
}

impl From<TransactionRequest> for TransactionArgs {
    fn from(val: TransactionRequest) -> Self {
        let note_args = val.get_note_args();
        TransactionArgs::new(val.tx_script, Some(note_args))
    }
}

// TRANSACTION TEMPLATE
// --------------------------------------------------------------------------------------------

#[derive(Clone)]
pub enum TransactionTemplate {
    /// Consume outstanding notes for an account.
    ConsumeNotes(AccountId, Vec<NoteId>),
    /// Mint fungible assets using a faucet account and creates a note that can be consumed by the target Account ID
    MintFungibleAsset(FungibleAsset, AccountId),
    /// Creates a pay-to-id note directed to a specific account
    PayToId(PaymentTransactionData),
    /// Creates a pay-to-id note directed to a specific account, specifying a block height after
    /// which the note can be recalled
    PayToIdWithRecall(PaymentTransactionData, u32),
}

impl TransactionTemplate {
    /// Returns the [AccountId] of the account which the transaction will be executed against
    pub fn account_id(&self) -> AccountId {
        match self {
            TransactionTemplate::ConsumeNotes(account_id, _) => *account_id,
            TransactionTemplate::MintFungibleAsset(asset, _) => asset.faucet_id(),
            TransactionTemplate::PayToId(payment_data) => payment_data.account_id(),
            TransactionTemplate::PayToIdWithRecall(payment_data, _) => payment_data.account_id(),
        }
    }
}

// PAYMENT TRANSACTION DATA
// --------------------------------------------------------------------------------------------

#[derive(Clone)]
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
