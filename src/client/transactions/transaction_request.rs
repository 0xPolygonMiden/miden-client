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
    /// Returns the executor [AccountId]
    pub fn account_id(&self) -> AccountId {
        match self {
            TransactionTemplate::ConsumeNotes(account_id, _) => *account_id,
            TransactionTemplate::MintFungibleAsset(asset, _) => asset.faucet_id(),
            TransactionTemplate::PayToId(p) => p.account_id(),
            TransactionTemplate::PayToIdWithRecall(p, _) => p.account_id(),
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

#[cfg(test)]
mod tests {
    

    use miden_lib::transaction::TransactionKernel;
    use miden_objects::{
        assembly::ProgramAst, utils::collections::BTreeMap, Felt,
        Word,
    };
    use miden_tx::utils::Serializable;

    use super::TransactionRequest;
    use crate::{
        client::accounts::{AccountStorageMode, AccountTemplate},
        mock::{
            mock_full_chain_mmr_and_notes, mock_notes,
            MockDataStore,
        },
        store::{sqlite_store::tests::create_test_client, AuthInfo},
    };

    #[tokio::test]
    async fn test_transaction_request() {
        let mut client = create_test_client();

        let account_template = AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Local,
        };

        // Insert Account
        let (account, seed) = client.new_account(account_template).unwrap();
        //client.sync_state().await.unwrap();

        // Prepare transaction

        let assembler = TransactionKernel::assembler();

        let (consumed_notes, created_notes) = mock_notes(&assembler);
        let (_mmr, consumed_notes, _tracked_block_headers, _mmr_deltas) =
            mock_full_chain_mmr_and_notes(consumed_notes);

        let note_args = [
            [Felt::new(91), Felt::new(91), Felt::new(91), Felt::new(91)],
            [Felt::new(92), Felt::new(92), Felt::new(92), Felt::new(92)],
        ];

        let _note_args_map = BTreeMap::from([
            (consumed_notes[0].id(), (note_args[1])),
            (consumed_notes[1].id(), (note_args[0])),
        ]);

        let code = "
        use.miden::tx
        use.miden::kernels::tx::prologue
        use.miden::kernels::tx::memory
        use.miden::kernels::tx::note

        begin
            exec.prologue::prepare_transaction
    
            # create output note 1
            #push.0
            #push.1
            #push.2
            #exec.tx::create_note
            #drop
    
            # create output note 2
            #push.3
            #push.4
            #push.5
            #exec.tx::create_note
            #drop

            #exec.memory::get_total_num_consumed_notes push.2 assert_eq
            #exec.note::prepare_note dropw
            #exec.note::increment_current_consumed_note_ptr drop
            #exec.note::prepare_note dropw
        end
        ";

        let program = ProgramAst::parse(code).unwrap();
        let mock_data_store = MockDataStore::new(account.clone(), Some(seed), None);
        client.set_data_store(mock_data_store);

        let tx_script = {
            let account_auth = client.get_account_auth(account.id()).unwrap();
            let (pubkey_input, advice_map): (Word, Vec<Felt>) = match account_auth {
                AuthInfo::RpoFalcon512(key) => (
                    key.public_key().into(),
                    key.to_bytes().iter().map(|a| Felt::new(*a as u64)).collect::<Vec<Felt>>(),
                ),
            };

            let _script_inputs = vec![(pubkey_input, advice_map)];

            client.tx_executor.compile_tx_script(program, vec![], vec![]).unwrap()
        };
        let expected_notes = vec![created_notes[0].clone(), created_notes[1].clone()];
        let transaction_request =
            TransactionRequest::new(account.id(), BTreeMap::new(), expected_notes, Some(tx_script));

        let execution = client.new_transaction(transaction_request);
        execution.unwrap();
    }
}
