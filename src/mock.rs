use async_trait::async_trait;
use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_node_proto::generated::{
    account::AccountId as ProtoAccountId,
    block_header::BlockHeader as NodeBlockHeader,
    note::NoteSyncRecord,
    requests::{GetBlockHeaderByNumberRequest, SyncStateRequest},
    responses::{NullifierUpdate, SyncStateResponse},
};
use miden_objects::{
    accounts::{
        get_account_seed_single, Account, AccountCode, AccountId, AccountStorage, AccountType,
        StorageSlotType,
    },
    assembly::{Assembler, ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset, TokenSymbol},
    crypto::{
        dsa::rpo_falcon512::KeyPair,
        merkle::{Mmr, MmrDelta, NodeIndex, SimpleSmt},
    },
    notes::{Note, NoteAssets, NoteInclusionProof, NoteScript},
    transaction::{InputNote, ProvenTransaction},
    utils::collections::BTreeMap,
    BlockHeader, Felt, Word, NOTE_TREE_DEPTH, ZERO,
};
use rand::Rng;
use tonic::{IntoRequest, Response, Status};

pub use crate::store::mock_executor_data_store::MockDataStore;
#[cfg(test)]
use crate::store::Store;
use crate::{
    client::{
        rpc::{NodeRpcClient, NodeRpcClientEndpoint, StateSyncInfo},
        sync::FILTER_ID_SHIFT,
        transactions::{prepare_word, PaymentTransactionData, TransactionTemplate},
        Client,
    },
    errors::NodeRpcClientError,
    store::{sqlite_store::SqliteStore, AuthInfo},
};

pub type MockClient = Client<MockRpcApi, SqliteStore>;

// MOCK CONSTS
// ================================================================================================

pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 3238098370154045919;
pub const ACCOUNT_ID_REGULAR: u64 = 0b0110111011u64 << 54;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;
pub const DEFAULT_ACCOUNT_CODE: &str = "
    use.miden::contracts::wallets::basic->basic_wallet
    use.miden::contracts::auth::basic->basic_eoa

    export.basic_wallet::receive_asset
    export.basic_wallet::send_asset
    export.basic_eoa::auth_tx_rpo_falcon512
";

/// Mock RPC API
///
/// This struct implements the RPC API used by the client to communicate with the node. It is
/// intended to be used for testing purposes only.
pub struct MockRpcApi {
    pub state_sync_requests: BTreeMap<SyncStateRequest, SyncStateResponse>,
}

impl Default for MockRpcApi {
    fn default() -> Self {
        Self {
            state_sync_requests: generate_state_sync_mock_requests(),
        }
    }
}

impl MockRpcApi {
    pub fn new(_config_endpoint: &str) -> Self {
        Self::default()
    }
}

#[async_trait]
impl NodeRpcClient for MockRpcApi {
    /// Executes the specified sync state request and returns the response.
    async fn sync_state(
        &mut self,
        block_num: u32,
        _account_ids: &[AccountId],
        _note_tags: &[u16],
        _nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, NodeRpcClientError> {
        // Match request -> response through block_num
        let response =
            match self.state_sync_requests.iter().find(|(req, _)| req.block_num == block_num) {
                Some((_req, response)) => {
                    let response = response.clone();
                    Ok(Response::new(response))
                },
                None => Err(NodeRpcClientError::RequestError(
                    NodeRpcClientEndpoint::SyncState.to_string(),
                    Status::not_found("no response for sync state request").to_string(),
                )),
            }?;

        response.into_inner().try_into()
    }

    /// Creates and executes a [GetBlockHeaderByNumberRequest].
    /// Only used for retrieving genesis block right now so that's the only case we need to cover.
    async fn get_block_header_by_number(
        &mut self,
        block_num: Option<u32>,
    ) -> Result<BlockHeader, NodeRpcClientError> {
        let request = GetBlockHeaderByNumberRequest { block_num };
        let request: GetBlockHeaderByNumberRequest = request.into_request().into_inner();

        if request.block_num == Some(0) {
            let block_header = BlockHeader::mock(0, None, None, &[]);
            return Ok(block_header);
        }
        panic!("get_block_header_by_number is supposed to be only used for genesis block")
    }

    async fn submit_proven_transaction(
        &mut self,
        _proven_transaction: ProvenTransaction,
    ) -> std::result::Result<(), NodeRpcClientError> {
        // TODO: add some basic validations to test error cases
        Ok(())
    }
}

// HELPERS
// ================================================================================================

/// Generates mock sync state requests and responses
fn create_mock_sync_state_request_for_account_and_notes(
    account_id: AccountId,
    output_notes: &[Note],
    consumed_notes: &[InputNote],
    mmr_delta: Option<Vec<MmrDelta>>,
    tracked_block_headers: Option<Vec<BlockHeader>>,
) -> BTreeMap<SyncStateRequest, SyncStateResponse> {
    let mut requests: BTreeMap<SyncStateRequest, SyncStateResponse> = BTreeMap::new();

    let accounts = vec![ProtoAccountId {
        id: u64::from(account_id),
    }];

    let nullifiers: Vec<u32> = consumed_notes
        .iter()
        .map(|note| (note.note().nullifier().as_elements()[3].as_int() >> FILTER_ID_SHIFT) as u32)
        .collect();

    let account = get_account_with_default_account_code(account_id, Word::default(), None);

    let tracked_block_headers = tracked_block_headers.unwrap_or(vec![
        BlockHeader::mock(8, None, None, &[]),
        BlockHeader::mock(10, None, None, &[]),
    ]);

    let chain_tip = tracked_block_headers.last().map(|header| header.block_num()).unwrap_or(10);
    let mut deltas_iter = mmr_delta.unwrap_or_default().into_iter();
    let mut created_notes_iter = output_notes.iter();

    for (block_order, block_header) in tracked_block_headers.iter().enumerate() {
        let request = SyncStateRequest {
            block_num: if block_order == 0 {
                0
            } else {
                tracked_block_headers[block_order - 1].block_num()
            },
            account_ids: accounts.clone(),
            note_tags: vec![],
            nullifiers: nullifiers.clone(),
        };

        // create a state sync response
        let response = SyncStateResponse {
            chain_tip,
            mmr_delta: deltas_iter.next().map(miden_node_proto::generated::mmr::MmrDelta::from),
            block_header: Some(NodeBlockHeader::from(*block_header)),
            accounts: vec![],
            notes: vec![NoteSyncRecord {
                note_index: 0,
                note_id: Some(created_notes_iter.next().unwrap().id().into()),
                sender: Some(account.id().into()),
                tag: 0u64,
                merkle_path: Some(miden_node_proto::generated::merkle::MerklePath::default()),
            }],
            nullifiers: vec![NullifierUpdate {
                nullifier: Some(consumed_notes.first().unwrap().note().nullifier().inner().into()),
                block_num: 7,
            }],
        };
        requests.insert(request, response);
    }

    requests
}

/// Generates mock sync state requests and responses
fn generate_state_sync_mock_requests() -> BTreeMap<SyncStateRequest, SyncStateResponse> {
    let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap();

    // create sync state requests
    let assembler = TransactionKernel::assembler();
    let (consumed_notes, created_notes) = mock_notes(&assembler);
    let (_, input_notes, _, _) = mock_full_chain_mmr_and_notes(consumed_notes);

    create_mock_sync_state_request_for_account_and_notes(
        account_id,
        &created_notes,
        &input_notes,
        None,
        None,
    )
}

pub fn mock_full_chain_mmr_and_notes(
    consumed_notes: Vec<Note>
) -> (Mmr, Vec<InputNote>, Vec<BlockHeader>, Vec<MmrDelta>) {
    let mut note_trees = Vec::new();

    // TODO: Consider how to better represent note authentication data.
    // we use the index for both the block number and the leaf index in the note tree
    for (index, note) in consumed_notes.iter().enumerate() {
        let tree_index = 2 * index;
        let smt_entries = vec![
            (tree_index as u64, note.id().into()),
            ((tree_index + 1) as u64, note.metadata().into()),
        ];
        let smt: SimpleSmt<NOTE_TREE_DEPTH> = SimpleSmt::with_leaves(smt_entries).unwrap();
        note_trees.push(smt);
    }

    let mut note_tree_iter = note_trees.iter();
    let mut mmr_deltas = Vec::new();

    // create a dummy chain of block headers
    let block_chain = vec![
        BlockHeader::mock(0, None, note_tree_iter.next().map(|x| x.root()), &[]),
        BlockHeader::mock(1, None, note_tree_iter.next().map(|x| x.root()), &[]),
        BlockHeader::mock(2, None, note_tree_iter.next().map(|x| x.root()), &[]),
        BlockHeader::mock(3, None, note_tree_iter.next().map(|x| x.root()), &[]),
        BlockHeader::mock(4, None, note_tree_iter.next().map(|x| x.root()), &[]),
        BlockHeader::mock(5, None, note_tree_iter.next().map(|x| x.root()), &[]),
        BlockHeader::mock(6, None, note_tree_iter.next().map(|x| x.root()), &[]),
    ];

    // instantiate and populate MMR
    let mut mmr = Mmr::default();
    for (block_num, block_header) in block_chain.iter().enumerate() {
        if block_num == 2 {
            mmr_deltas.push(mmr.get_delta(1, mmr.forest()).unwrap());
        }
        if block_num == 4 {
            mmr_deltas.push(mmr.get_delta(3, mmr.forest()).unwrap());
        }
        if block_num == 6 {
            mmr_deltas.push(mmr.get_delta(5, mmr.forest()).unwrap());
        }
        mmr.add(block_header.hash());
    }

    // set origin for consumed notes using chain and block data
    let recorded_notes = consumed_notes
        .into_iter()
        .enumerate()
        .map(|(index, note)| {
            let block_header = &block_chain[index];
            let auth_index = NodeIndex::new(NOTE_TREE_DEPTH, index as u64).unwrap();
            InputNote::new(
                note,
                NoteInclusionProof::new(
                    block_header.block_num(),
                    block_header.sub_hash(),
                    block_header.note_root(),
                    index as u64,
                    note_trees[index].open(&auth_index.try_into().unwrap()).path,
                )
                .unwrap(),
            )
        })
        .collect::<Vec<_>>();

    (
        mmr,
        recorded_notes,
        vec![block_chain[2], block_chain[4], block_chain[6]],
        mmr_deltas,
    )
}

/// inserts mock note and account data into the client and returns the last block header of mocked
/// chain
pub async fn insert_mock_data(client: &mut MockClient) -> Vec<BlockHeader> {
    // mock notes
    let account = get_account_with_default_account_code(
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap(),
        Word::default(),
        None,
    );

    let init_seed: [u8; 32] = [0; 32];
    let account_seed = get_account_seed_single(
        init_seed,
        account.account_type(),
        true,
        account.code().root(),
        account.storage().root(),
    )
    .unwrap();

    let assembler = TransactionKernel::assembler();
    let (consumed_notes, created_notes) = mock_notes(&assembler);
    let (_mmr, consumed_notes, tracked_block_headers, mmr_deltas) =
        mock_full_chain_mmr_and_notes(consumed_notes);

    // insert notes into database
    for note in consumed_notes.clone() {
        client.import_input_note(note.into()).unwrap();
    }

    // insert notes into database
    for note in created_notes.clone() {
        client.import_input_note(note.into()).unwrap();
    }

    // insert account
    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();
    client
        .insert_account(&account, Some(account_seed), &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();

    client.rpc_api().state_sync_requests = create_mock_sync_state_request_for_account_and_notes(
        account.id(),
        &created_notes,
        &consumed_notes,
        Some(mmr_deltas),
        Some(tracked_block_headers.clone()),
    );

    tracked_block_headers
}

pub async fn create_mock_transaction(client: &mut MockClient) {
    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();
    let auth_scheme: miden_lib::AuthScheme = miden_lib::AuthScheme::RpoFalcon512 {
        pub_key: key_pair.public_key(),
    };

    let mut rng = rand::thread_rng();
    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = rand::Rng::gen(&mut rng);

    let (sender_account, seed) = miden_lib::accounts::wallets::create_basic_wallet(
        init_seed,
        auth_scheme,
        AccountType::RegularAccountImmutableCode,
    )
    .unwrap();

    client
        .insert_account(&sender_account, Some(seed), &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();

    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();
    let auth_scheme: miden_lib::AuthScheme = miden_lib::AuthScheme::RpoFalcon512 {
        pub_key: key_pair.public_key(),
    };

    let mut rng = rand::thread_rng();
    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = rand::Rng::gen(&mut rng);

    let (target_account, seed) = miden_lib::accounts::wallets::create_basic_wallet(
        init_seed,
        auth_scheme,
        AccountType::RegularAccountImmutableCode,
    )
    .unwrap();

    client
        .insert_account(&target_account, Some(seed), &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();

    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();
    let auth_scheme: miden_lib::AuthScheme = miden_lib::AuthScheme::RpoFalcon512 {
        pub_key: key_pair.public_key(),
    };

    let mut rng = rand::thread_rng();
    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = rand::Rng::gen(&mut rng);

    let max_supply = 10000u64.to_le_bytes();

    let (faucet, seed) = miden_lib::accounts::faucets::create_basic_fungible_faucet(
        init_seed,
        miden_objects::assets::TokenSymbol::new("MOCK").unwrap(),
        4u8,
        Felt::try_from(max_supply.as_slice()).unwrap(),
        auth_scheme,
    )
    .unwrap();

    client
        .insert_account(&faucet, Some(seed), &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();

    let asset: miden_objects::assets::Asset = FungibleAsset::new(faucet.id(), 5u64).unwrap().into();

    // Insert a P2ID transaction object

    let transaction_template = TransactionTemplate::PayToId(PaymentTransactionData::new(
        asset,
        sender_account.id(),
        target_account.id(),
    ));

    let transaction_execution_result = client.new_transaction(transaction_template).unwrap();

    client.send_transaction(transaction_execution_result).await.unwrap();
}

pub fn mock_fungible_faucet_account(
    id: AccountId,
    initial_balance: u64,
    key_pair: KeyPair,
) -> Account {
    let mut rng = rand::thread_rng();
    let init_seed: [u8; 32] = rng.gen();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
        pub_key: key_pair.public_key(),
    };

    let (faucet, _seed) = miden_lib::accounts::faucets::create_basic_fungible_faucet(
        init_seed,
        TokenSymbol::new("TST").unwrap(),
        10u8,
        Felt::try_from(initial_balance.to_le_bytes().as_slice())
            .expect("u64 can be safely converted to a field element"),
        auth_scheme,
    )
    .unwrap();

    let faucet_storage_slot_1 =
        [Felt::new(initial_balance), Felt::new(0), Felt::new(0), Felt::new(0)];
    let faucet_account_storage = AccountStorage::new(vec![
        (0, (StorageSlotType::Value { value_arity: 0 }, key_pair.public_key().into())),
        (1, (StorageSlotType::Value { value_arity: 0 }, faucet_storage_slot_1)),
    ])
    .unwrap();

    Account::new(
        id,
        AssetVault::new(&[]).unwrap(),
        faucet_account_storage.clone(),
        faucet.code().clone(),
        Felt::new(10u64),
    )
}

#[cfg(test)]
impl<N: NodeRpcClient, S: Store> Client<N, S> {
    /// Helper function to set a data store to conveniently mock data for tests
    pub fn set_data_store(
        &mut self,
        data_store: MockDataStore,
    ) {
        self.set_tx_executor(miden_tx::TransactionExecutor::new(data_store));
    }
}

pub fn mock_notes(assembler: &Assembler) -> (Vec<Note>, Vec<Note>) {
    const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
        0b1010010001111111010110100011011110101011010001101111110110111100u64;
    const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u64 =
        0b1010000101101010101101000110111101010110100011011110100011011101u64;
    const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u64 =
        0b1010011001011010101101000110111101010110100011011101000110111100u64;
    // Note Assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 150).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 7).unwrap().into();

    // Sender account
    let sender = AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap();

    // CREATED NOTES
    // --------------------------------------------------------------------------------------------
    // create note script
    let note_program_ast = ProgramAst::parse("begin push.1 drop end").unwrap();
    let (note_script, _) = NoteScript::new(note_program_ast, assembler).unwrap();

    // Created Notes
    const SERIAL_NUM_4: Word = [Felt::new(13), Felt::new(14), Felt::new(15), Felt::new(16)];
    let created_note_1 = Note::new(
        note_script.clone(),
        &[Felt::new(1)],
        &[fungible_asset_1],
        SERIAL_NUM_4,
        sender,
        ZERO,
    )
    .unwrap();

    const SERIAL_NUM_5: Word = [Felt::new(17), Felt::new(18), Felt::new(19), Felt::new(20)];
    let created_note_2 = Note::new(
        note_script.clone(),
        &[Felt::new(2)],
        &[fungible_asset_2],
        SERIAL_NUM_5,
        sender,
        ZERO,
    )
    .unwrap();

    const SERIAL_NUM_6: Word = [Felt::new(21), Felt::new(22), Felt::new(23), Felt::new(24)];
    let created_note_3 =
        Note::new(note_script, &[Felt::new(2)], &[fungible_asset_3], SERIAL_NUM_6, sender, ZERO)
            .unwrap();

    let created_notes = vec![created_note_1, created_note_2, created_note_3];

    // CONSUMED NOTES
    // --------------------------------------------------------------------------------------------

    // create note 1 script
    let note_1_script_src = format!(
        "\
        begin
            # create note 0
            push.{created_note_0_recipient}
            push.{created_note_0_tag}
            push.{created_note_0_asset}
            # MAST root of the `create_note` mock account procedure
            # call.0xacb46cadec8d1721934827ed161b851f282f1f4b88b72391a67fed668b1a00ba
            drop dropw dropw

            # create note 1
            push.{created_note_1_recipient}
            push.{created_note_1_tag}
            push.{created_note_1_asset}
            # MAST root of the `create_note` mock account procedure
            # call.0xacb46cadec8d1721934827ed161b851f282f1f4b88b72391a67fed668b1a00ba
            drop dropw dropw
        end
    ",
        created_note_0_recipient = prepare_word(&created_notes[0].recipient()),
        created_note_0_tag = created_notes[0].metadata().tag(),
        created_note_0_asset = prepare_assets(created_notes[0].assets())[0],
        created_note_1_recipient = prepare_word(&created_notes[1].recipient()),
        created_note_1_tag = created_notes[1].metadata().tag(),
        created_note_1_asset = prepare_assets(created_notes[1].assets())[0],
    );
    let note_1_script_ast = ProgramAst::parse(&note_1_script_src).unwrap();
    let (note_1_script, _) = NoteScript::new(note_1_script_ast, assembler).unwrap();

    // create note 2 script
    let note_2_script_src = format!(
        "\
        begin
            # create note 2
            push.{created_note_2_recipient}
            push.{created_note_2_tag}
            push.{created_note_2_asset}
            # MAST root of the `create_note` mock account procedure
            # call.0xacb46cadec8d1721934827ed161b851f282f1f4b88b72391a67fed668b1a00ba
            drop dropw dropw
        end
        ",
        created_note_2_recipient = prepare_word(&created_notes[2].recipient()),
        created_note_2_tag = created_notes[2].metadata().tag(),
        created_note_2_asset = prepare_assets(created_notes[2].assets())[0],
    );
    let note_2_script_ast = ProgramAst::parse(&note_2_script_src).unwrap();
    let (note_2_script, _) = NoteScript::new(note_2_script_ast, assembler).unwrap();

    // Consumed Notes
    const SERIAL_NUM_1: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let consumed_note_1 =
        Note::new(note_1_script, &[Felt::new(1)], &[fungible_asset_1], SERIAL_NUM_1, sender, ZERO)
            .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
    let consumed_note_2 = Note::new(
        note_2_script,
        &[Felt::new(2)],
        &[fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
        sender,
        ZERO,
    )
    .unwrap();

    let consumed_notes = vec![consumed_note_1, consumed_note_2];

    (consumed_notes, created_notes)
}

fn get_account_with_nonce(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
    nonce: u64,
) -> Account {
    let account_code_src = DEFAULT_ACCOUNT_CODE;
    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let account_code = AccountCode::new(account_code_ast, &account_assembler).unwrap();
    let account_storage =
        AccountStorage::new(vec![(0, (StorageSlotType::Value { value_arity: 0 }, public_key))])
            .unwrap();

    let asset_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::new(account_id, asset_vault, account_storage, account_code, Felt::new(nonce))
}

pub fn get_account_with_default_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    get_account_with_nonce(account_id, public_key, assets, 1)
}

pub fn get_new_account_with_default_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    get_account_with_nonce(account_id, public_key, assets, 0)
}

fn prepare_assets(note_assets: &NoteAssets) -> Vec<String> {
    let mut assets = Vec::new();
    for &asset in note_assets.iter() {
        let asset_word: Word = asset.into();
        let asset_str = prepare_word(&asset_word);
        assets.push(asset_str);
    }
    assets
}
