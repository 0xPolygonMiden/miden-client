use assembly::{Library, LibraryPath};
use miden_lib::{
    transaction::{memory::FAUCET_STORAGE_DATA_SLOT, TransactionKernel},
    MidenLib,
};
use miden_tx::{DataStore, DataStoreError, TransactionInputs};
use mock::{
    constants::{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_SENDER, DEFAULT_ACCOUNT_CODE},
    mock::{
        account::MockAccountType,
        notes::AssetPreservationStatus,
        transaction::{mock_inputs, mock_inputs_with_existing},
    },
};
use objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, StorageSlotType},
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::{dsa::rpo_falcon512::KeyPair, utils::Serializable},
    notes::{Note, NoteId, NoteScript},
    transaction::{ChainMmr, InputNotes},
    BlockHeader, Felt, Word,
};

// MOCK DATA STORE
// ================================================================================================

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub input_notes: InputNotes,
}

impl MockDataStore {
    pub fn new() -> Self {
        let transaction_data = mock_inputs(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
        );
        Self {
            account: transaction_data.account().clone(),
            block_header: *transaction_data.block_header(),
            block_chain: transaction_data.block_chain().clone(),
            input_notes: transaction_data.input_notes().clone(),
        }
    }

    pub fn with_existing(account: Account, consumed_notes: Option<Vec<Note>>) -> Self {
        let (_mocked_account, block_header, block_chain, consumed_notes, _auxiliary_data_inputs) =
            // NOTE: Currently this disregards the mocked account and uses the passed account
            mock_inputs_with_existing(
                MockAccountType::StandardExisting,
                AssetPreservationStatus::Preserved,
                Some(account.clone()),
                consumed_notes,
            );

        Self {
            account,
            block_header,
            block_chain,
            input_notes: InputNotes::new(consumed_notes).unwrap(),
        }
    }
}

impl Default for MockDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DataStore for MockDataStore {
    /// NOTE: This method assumes the MockDataStore was created accordingly using `with_existing()`
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        _block_num: u32,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        let origins = self
            .input_notes
            .iter()
            .map(|note| note.id())
            .collect::<Vec<_>>();
        notes.iter().all(|note| origins.contains(note));
        TransactionInputs::new(
            self.account.clone(),
            None,
            self.block_header,
            self.block_chain.clone(),
            self.input_notes.clone(),
        )
        .map_err(|_err| DataStoreError::AccountNotFound(account_id))
    }

    fn get_account_code(&self, _account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        Ok(self.account.code().module().clone())
    }
}

// HELPER FUNCTIONS
// ================================================================================================
pub fn get_new_key_pair_with_advice_map() -> (Word, Vec<Felt>) {
    let keypair: KeyPair = KeyPair::new().unwrap();

    let pk: Word = keypair.public_key().into();
    let pk_sk_bytes = keypair.to_bytes();
    let pk_sk_felts: Vec<Felt> = pk_sk_bytes
        .iter()
        .map(|a| Felt::new(*a as u64))
        .collect::<Vec<Felt>>();

    (pk, pk_sk_felts)
}

#[allow(dead_code)]
pub fn get_account_with_default_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let account_code_src = DEFAULT_ACCOUNT_CODE;
    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler).unwrap();
    let account_storage = AccountStorage::new(vec![(
        0,
        (StorageSlotType::Value { value_arity: 0 }, public_key),
    )])
    .unwrap();

    let asset_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::new(
        account_id,
        asset_vault,
        account_storage,
        account_code,
        Felt::new(1),
    )
}

#[allow(dead_code)]
pub fn get_note_with_fungible_asset_and_script(
    fungible_asset: FungibleAsset,
    note_script: ProgramAst,
) -> Note {
    let note_assembler = TransactionKernel::assembler();

    let (note_script, _) = NoteScript::new(note_script, &note_assembler).unwrap();
    const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let sender_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    Note::new(
        note_script.clone(),
        &[],
        &[fungible_asset.into()],
        SERIAL_NUM,
        sender_id,
        Felt::new(1),
    )
    .unwrap()
}

pub fn get_faucet_account_with_max_supply_and_total_issuance(
    public_key: Word,
    max_supply: u64,
    total_issuance: Option<u64>,
) -> Account {
    let faucet_account_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();

    let miden = MidenLib::default();
    let path = "miden::contracts::faucets::basic_fungible";
    let faucet_code_ast = miden
        .get_module_ast(&LibraryPath::new(path).unwrap())
        .expect("Getting module AST failed");

    let account_assembler = TransactionKernel::assembler();
    let _account_code = AccountCode::new(faucet_code_ast.clone(), &account_assembler).unwrap();

    let faucet_account_code =
        AccountCode::new(faucet_code_ast.clone(), &account_assembler).unwrap();

    let faucet_storage_slot_1 = [
        Felt::new(max_supply),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ];
    let mut faucet_account_storage = AccountStorage::new(vec![
        (0, (StorageSlotType::Value { value_arity: 0 }, public_key)),
        (
            1,
            (
                StorageSlotType::Value { value_arity: 0 },
                faucet_storage_slot_1,
            ),
        ),
    ])
    .unwrap();

    if let Some(total_issuance) = total_issuance {
        let faucet_storage_slot_254 = [
            Felt::new(0),
            Felt::new(0),
            Felt::new(0),
            Felt::new(total_issuance),
        ];
        faucet_account_storage
            .set_item(FAUCET_STORAGE_DATA_SLOT, faucet_storage_slot_254)
            .unwrap();
    };

    Account::new(
        faucet_account_id,
        AssetVault::new(&[]).unwrap(),
        faucet_account_storage.clone(),
        faucet_account_code.clone(),
        Felt::new(1),
    )
}
