use crypto::merkle::PartialMmr;
use miden_lib::{
    transaction::{memory::FAUCET_STORAGE_DATA_SLOT, TransactionKernel},
    MidenLib,
};
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, StorageSlotType},
    assembly::{Library, LibraryPath, ModuleAst, ProgramAst},
    assets::{AssetVault, FungibleAsset},
    crypto::{dsa::rpo_falcon512::KeyPair, utils::Serializable},
    notes::{Note, NoteId, NoteScript},
    transaction::{ChainMmr, InputNote, InputNotes},
    BlockHeader, Felt, Word,
};
use miden_tx::{DataStore, DataStoreError, TransactionInputs};

use crate::mock::{
    get_account_with_default_account_code, mock_full_chain_mmr_and_notes,
    ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_REGULAR,
};

// MOCK DATA STORE
// ================================================================================================

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub account_seed: Option<Word>,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub input_notes: InputNotes,
}

impl MockDataStore {
    pub fn new(
        account: Account,
        account_seed: Option<Word>,
        input_notes: Option<Vec<InputNote>>,
    ) -> Self {
        let (mmr, _notes, headers, _) = mock_full_chain_mmr_and_notes(vec![]);
        let partial_mmr_peaks = mmr.peaks(mmr.forest()).unwrap();
        let mut partial_mmr = PartialMmr::from_peaks(partial_mmr_peaks);

        for block in headers.iter() {
            let merkle_path = mmr
                .open(block.block_num() as usize, mmr.forest())
                .unwrap()
                .merkle_path;
            partial_mmr
                .track(block.block_num() as usize, block.hash(), &merkle_path)
                .unwrap();
        }

        Self {
            account,
            // NOTE: This last block header is ahead of the mocked chain MMR view in order to correctly build transaction inputs
            block_header: BlockHeader::mock(
                7,
                Some(mmr.peaks(mmr.forest()).unwrap().hash_peaks()),
                None,
                &[],
            ),
            block_chain: ChainMmr::new(partial_mmr, headers).unwrap(),
            input_notes: InputNotes::new(input_notes.unwrap_or_default()).unwrap(),
            account_seed,
        }
    }
}

impl Default for MockDataStore {
    fn default() -> Self {
        let account = get_account_with_default_account_code(
            ACCOUNT_ID_REGULAR.try_into().unwrap(),
            Word::default(),
            None,
        );
        Self::new(account, Some(Word::default()), None)
    }
}

impl DataStore for MockDataStore {
    /// NOTE: This method assumes the MockDataStore was created accordingly using `with_existing()`
    fn get_transaction_inputs(
        &self,
        _account_id: AccountId,
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
            self.account_seed,
            self.block_header,
            self.block_chain.clone(),
            self.input_notes.clone(),
        )
        .map_err(|err| DataStoreError::InternalError(err.to_string()))
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
pub fn get_note_with_fungible_asset_and_script(
    fungible_asset: FungibleAsset,
    note_script: ProgramAst,
) -> Note {
    let note_assembler = TransactionKernel::assembler();
    const ACCOUNT_ID_SENDER: u64 = 0b0110111011u64 << 54;

    let (note_script, _) = NoteScript::new(note_script, &note_assembler).unwrap();
    const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let sender_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    Note::new(
        note_script,
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
