use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use std::{env::temp_dir, rc::Rc};

use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_OFF_CHAIN_SENDER, get_account_seed_single, Account,
        AccountCode, AccountId, AccountStorage, AccountStorageMode, AccountType, AuthSecretKey,
        SlotItem, StorageSlot,
    },
    assembly::Assembler,
    assets::{Asset, AssetVault, FungibleAsset, TokenSymbol},
    block::{BlockNoteIndex, BlockNoteTree},
    crypto::{
        dsa::rpo_falcon512::SecretKey,
        merkle::{Mmr, MmrDelta, MmrProof},
        rand::RpoRandomCoin,
    },
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteFile, NoteId, NoteInclusionProof, NoteInputs,
        NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    transaction::{InputNote, ProvenTransaction},
    BlockHeader, Felt, Word,
};
use rand::Rng;
use tonic::{Response, Status};
use uuid::Uuid;

use crate::{
    config::RpcConfig,
    rpc::{
        generated::{
            self,
            account::AccountId as ProtoAccountId,
            block::BlockHeader as NodeBlockHeader,
            note::NoteSyncRecord,
            requests::SyncStateRequest,
            responses::{NullifierUpdate, SyncNoteResponse, SyncStateResponse},
        },
        AccountDetails, NodeRpcClient, NodeRpcClientEndpoint, NoteDetails, NoteInclusionDetails,
        RpcError, StateSyncInfo,
    },
    store::{
        sqlite_store::{config::SqliteStoreConfig, SqliteStore},
        InputNoteRecord,
    },
    store_authenticator::StoreAuthenticator,
    sync::get_nullifier_prefix,
    transactions::{prepare_word, PaymentTransactionData, TransactionRequest},
    Client,
};

pub type MockClient =
    Client<MockRpcApi, RpoRandomCoin, SqliteStore, StoreAuthenticator<RpoRandomCoin, SqliteStore>>;

// MOCK CONSTS
// ================================================================================================

pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 3238098370154045919;
pub const ACCOUNT_ID_REGULAR: u64 = ACCOUNT_ID_OFF_CHAIN_SENDER;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;
pub const DEFAULT_ACCOUNT_CODE: &str = "
    export.::miden::contracts::wallets::basic::receive_asset
    export.::miden::contracts::wallets::basic::create_note
    export.::miden::contracts::wallets::basic::move_asset_to_note
    export.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
";

/// Mock RPC API
///
/// This struct implements the RPC API used by the client to communicate with the node. It is
/// intended to be used for testing purposes only.
pub struct MockRpcApi {
    pub state_sync_requests: BTreeMap<u32, SyncStateResponse>,
    pub genesis_block: BlockHeader,
    pub notes: BTreeMap<NoteId, InputNote>,
    pub mmr: Mmr,
    pub blocks: Vec<BlockHeader>,
    pub sync_note_request: SyncNoteResponse,
}

impl Default for MockRpcApi {
    fn default() -> Self {
        let (genesis_block, state_sync_requests, notes, mmr, blocks) =
            generate_state_sync_mock_requests();

        let sync_note_request = SyncNoteResponse {
            chain_tip: 10,
            notes: vec![],
            block_header: Some(BlockHeader::mock(1, None, None, &[]).into()),
            mmr_path: Some(Default::default()),
        };

        Self {
            state_sync_requests,
            genesis_block,
            notes,
            mmr,
            blocks,
            sync_note_request,
        }
    }
}

impl MockRpcApi {
    pub fn new(_config_endpoint: &str) -> Self {
        Self::default()
    }
}

impl NodeRpcClient for MockRpcApi {
    async fn sync_notes(
        &mut self,
        _block_num: u32,
        _note_tags: &[NoteTag],
    ) -> Result<crate::rpc::NoteSyncInfo, RpcError> {
        let response = &self.sync_note_request;
        let response = Response::new(response.clone());
        response.into_inner().try_into()
    }

    /// Executes the specified sync state request and returns the response.
    async fn sync_state(
        &mut self,
        block_num: u32,
        _account_ids: &[AccountId],
        _note_tags: &[NoteTag],
        _nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, RpcError> {
        // Match request -> response through block_num
        let response = match self.state_sync_requests.get(&block_num) {
            Some(response) => {
                let response = response.clone();
                Ok(Response::new(response))
            },
            None => Err(RpcError::RequestError(
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
        include_mmr_proof: bool,
    ) -> Result<(BlockHeader, Option<MmrProof>), RpcError> {
        if block_num == Some(0) {
            return Ok((self.genesis_block, None));
        }
        let block = self.blocks.iter().find(|b| b.block_num() == block_num.unwrap()).unwrap();

        let mmr_proof = if include_mmr_proof {
            Some(self.mmr.open(block_num.unwrap() as usize, self.mmr.forest()).unwrap())
        } else {
            None
        };

        Ok((*block, mmr_proof))
    }

    async fn get_notes_by_id(&mut self, note_ids: &[NoteId]) -> Result<Vec<NoteDetails>, RpcError> {
        // assume all off-chain notes for now
        let hit_notes = note_ids.iter().filter_map(|id| self.notes.get(id));
        let mut return_notes = vec![];
        for note in hit_notes {
            if note.note().metadata().note_type() != NoteType::Private {
                panic!("this function assumes all notes are offchain for now");
            }
            let inclusion_details = NoteInclusionDetails::new(
                note.proof()
                    .expect("Note should have an inclusion proof")
                    .location()
                    .block_num(),
                note.proof()
                    .expect("Note should have an inclusion proof")
                    .location()
                    .node_index_in_block(),
                note.proof().expect("Note should have an inclusion proof").note_path().clone(),
            );
            return_notes.push(NoteDetails::Private(
                note.id(),
                *note.note().metadata(),
                inclusion_details,
            ));
        }
        Ok(return_notes)
    }

    async fn submit_proven_transaction(
        &mut self,
        _proven_transaction: ProvenTransaction,
    ) -> std::result::Result<(), RpcError> {
        // TODO: add some basic validations to test error cases
        Ok(())
    }

    async fn get_account_update(
        &mut self,
        _account_id: AccountId,
    ) -> Result<AccountDetails, RpcError> {
        panic!("shouldn't be used for now")
    }

    async fn check_nullifiers_by_prefix(
        &mut self,
        _prefix: &[u16],
    ) -> Result<Vec<(miden_objects::notes::Nullifier, u32)>, RpcError> {
        // Always return an empty list for now since it's only used when importing
        Ok(vec![])
    }
}

// HELPERS
// ================================================================================================

/// Generates genesis block header, mock sync state requests and responses
fn create_mock_sync_state_request_for_account_and_notes(
    account_id: AccountId,
    output_notes: &[Note],
    consumed_notes: &[InputNote],
    genesis_block: &BlockHeader,
    mmr_delta: Option<Vec<MmrDelta>>,
    tracked_block_headers: Option<Vec<BlockHeader>>,
) -> BTreeMap<u32, SyncStateResponse> {
    let mut requests: BTreeMap<u32, SyncStateResponse> = BTreeMap::new();

    let accounts = vec![ProtoAccountId { id: u64::from(account_id) }];

    let nullifiers: Vec<u32> = consumed_notes
        .iter()
        .map(|note| get_nullifier_prefix(&note.note().nullifier()) as u32)
        .collect();

    let account = get_account_with_default_account_code(account_id, Word::default(), None);

    // This assumes the callee provides either both `tracked_block_headers` and `mmr_delta` are
    // provided or not provided
    let (tracked_block_headers, mmr_delta) =
        if let Some(tracked_block_headers) = tracked_block_headers {
            (tracked_block_headers, mmr_delta.unwrap())
        } else {
            let mut mocked_tracked_headers =
                vec![BlockHeader::mock(8, None, None, &[]), BlockHeader::mock(10, None, None, &[])];

            let all_mocked_block_headers = vec![
                *genesis_block,
                BlockHeader::mock(1, None, None, &[]),
                BlockHeader::mock(2, None, None, &[]),
                BlockHeader::mock(3, None, None, &[]),
                BlockHeader::mock(4, None, None, &[]),
                BlockHeader::mock(5, None, None, &[]),
                BlockHeader::mock(6, None, None, &[]),
                BlockHeader::mock(7, None, None, &[]),
                mocked_tracked_headers[0],
                BlockHeader::mock(9, None, None, &[]),
                mocked_tracked_headers[1],
            ];

            let mut mmr = Mmr::default();
            let mut mocked_mmr_deltas = vec![];

            for (block_num, block_header) in all_mocked_block_headers.iter().enumerate() {
                if block_num == 8 {
                    mocked_mmr_deltas.push(mmr.get_delta(1, mmr.forest()).unwrap());
                }
                if block_num == 10 {
                    // Fix mocked block chain root
                    mocked_tracked_headers[1] = BlockHeader::mock(
                        10,
                        Some(mmr.peaks(mmr.forest()).unwrap().hash_peaks()),
                        None,
                        &[],
                    );
                    mocked_mmr_deltas.push(mmr.get_delta(9, mmr.forest()).unwrap());
                }
                mmr.add(block_header.hash());
            }

            (mocked_tracked_headers, mocked_mmr_deltas)
        };

    let chain_tip = tracked_block_headers.last().map(|header| header.block_num()).unwrap_or(10);
    let mut deltas_iter = mmr_delta.into_iter();
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

        let metadata = generated::note::NoteMetadata {
            sender: Some(account.id().into()),
            note_type: NoteType::Private as u32,
            execution_hint: NoteExecutionHint::none().into(),
            tag: NoteTag::for_local_use_case(1u16, 0u16).unwrap().into(),
            aux: Default::default(),
        };

        // create a state sync response
        let response = SyncStateResponse {
            chain_tip,
            mmr_delta: deltas_iter.next().map(generated::mmr::MmrDelta::from),
            block_header: Some(NodeBlockHeader::from(*block_header)),
            accounts: vec![],
            notes: vec![NoteSyncRecord {
                note_index: 0,
                note_id: Some(created_notes_iter.next().unwrap().id().into()),
                metadata: Some(metadata),
                merkle_path: Some(generated::merkle::MerklePath::default()),
            }],
            nullifiers: vec![NullifierUpdate {
                nullifier: Some(consumed_notes.first().unwrap().note().nullifier().inner().into()),
                block_num: 7,
            }],
            transactions: vec![],
        };
        requests.insert(request.block_num, response);
    }

    requests
}

/// Generates mock sync state requests and responses
#[allow(clippy::type_complexity)]
fn generate_state_sync_mock_requests() -> (
    BlockHeader,
    BTreeMap<u32, SyncStateResponse>,
    BTreeMap<NoteId, InputNote>,
    Mmr,
    Vec<BlockHeader>,
) {
    let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR).unwrap();

    // create sync state requests
    let assembler = TransactionKernel::assembler();
    let (consumed_notes, created_notes) = mock_notes(assembler);
    let (mmr, input_notes, blocks, ..) = mock_full_chain_mmr_and_notes(consumed_notes);

    let genesis_block = BlockHeader::mock(0, None, None, &[]);

    let state_sync_request_responses = create_mock_sync_state_request_for_account_and_notes(
        account_id,
        &created_notes,
        &input_notes,
        &genesis_block,
        None,
        None,
    );
    let input_notes = input_notes.iter().map(|n| (n.note().id(), n.clone())).collect();
    (genesis_block, state_sync_request_responses, input_notes, mmr, blocks)
}

pub fn mock_full_chain_mmr_and_notes(
    consumed_notes: Vec<Note>,
) -> (Mmr, Vec<InputNote>, Vec<BlockHeader>, Vec<MmrDelta>) {
    // TODO: Consider how to better represent note authentication data.
    // we use the index for both the block number and the leaf index in the note tree
    let tree_entries = consumed_notes
        .iter()
        .enumerate()
        .map(|(index, note)| (BlockNoteIndex::new(1, index), note.id().into(), *note.metadata()));
    let note_tree = BlockNoteTree::with_entries(tree_entries).unwrap();

    let mut mmr_deltas = Vec::new();

    // create a dummy chain of block headers
    let block_chain = vec![
        BlockHeader::mock(0, None, None, &[]),
        BlockHeader::mock(1, None, None, &[]),
        BlockHeader::mock(2, None, Some(note_tree.root()), &[]),
        BlockHeader::mock(3, None, None, &[]),
        BlockHeader::mock(4, None, None, &[]),
        BlockHeader::mock(5, None, None, &[]),
        BlockHeader::mock(6, None, None, &[]),
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
            let block_header = &block_chain[2];
            InputNote::authenticated(
                note,
                NoteInclusionProof::new(
                    block_header.block_num(),
                    index.try_into().unwrap(),
                    note_tree.get_note_path(BlockNoteIndex::new(1, index)).unwrap(),
                )
                .unwrap(),
            )
        })
        .collect::<Vec<_>>();

    (mmr, recorded_notes, block_chain, mmr_deltas)
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
        miden_objects::accounts::AccountStorageMode::Private,
        account.code().commitment(),
        account.storage().root(),
    )
    .unwrap();

    let assembler = TransactionKernel::assembler();
    let (consumed_notes, created_notes) = mock_notes(assembler);
    let (_mmr, consumed_notes, tracked_block_headers, mmr_deltas) =
        mock_full_chain_mmr_and_notes(consumed_notes);

    let tracked_block_headers =
        vec![tracked_block_headers[2], tracked_block_headers[4], tracked_block_headers[6]];

    // insert notes into database
    for note in consumed_notes.clone() {
        let note: InputNoteRecord = note.into();
        client
            .import_note(NoteFile::NoteWithProof(
                note.clone().try_into().unwrap(),
                note.inclusion_proof().unwrap().clone(),
            ))
            .await
            .unwrap();
    }

    // insert notes into database
    for note in created_notes.clone() {
        let note: InputNoteRecord = note.into();
        let tag = note.metadata().unwrap().tag();
        client.add_note_tag(tag).unwrap();
        client
            .import_note(NoteFile::NoteDetails {
                details: note.into(),
                tag: Some(tag),
                after_block_num: 0,
            })
            .await
            .unwrap();
    }

    // insert account
    let secret_key = SecretKey::new();
    client
        .insert_account(&account, Some(account_seed), &AuthSecretKey::RpoFalcon512(secret_key))
        .unwrap();

    let genesis_block = BlockHeader::mock(0, None, None, &[]);

    client.rpc_api().state_sync_requests = create_mock_sync_state_request_for_account_and_notes(
        account.id(),
        &created_notes,
        &consumed_notes,
        &genesis_block,
        Some(mmr_deltas),
        Some(tracked_block_headers.clone()),
    );

    tracked_block_headers
}

pub async fn create_mock_transaction(client: &mut MockClient) {
    let key_pair = SecretKey::new();
    let auth_scheme: miden_lib::AuthScheme =
        miden_lib::AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

    let mut rng = rand::thread_rng();
    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = rand::Rng::gen(&mut rng);

    let (sender_account, seed) = miden_lib::accounts::wallets::create_basic_wallet(
        init_seed,
        auth_scheme,
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Private,
    )
    .unwrap();

    client
        .insert_account(&sender_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();

    let key_pair = SecretKey::new();
    let auth_scheme: miden_lib::AuthScheme =
        miden_lib::AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

    let mut rng = rand::thread_rng();
    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = rand::Rng::gen(&mut rng);

    let (target_account, seed) = miden_lib::accounts::wallets::create_basic_wallet(
        init_seed,
        auth_scheme,
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Private,
    )
    .unwrap();

    client
        .insert_account(&target_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();

    let key_pair = SecretKey::new();
    let auth_scheme: miden_lib::AuthScheme =
        miden_lib::AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

    let mut rng = rand::thread_rng();
    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = rand::Rng::gen(&mut rng);

    let max_supply = 10000u64.to_le_bytes();

    let (faucet, seed) = miden_lib::accounts::faucets::create_basic_fungible_faucet(
        init_seed,
        miden_objects::assets::TokenSymbol::new("MOCK").unwrap(),
        4u8,
        Felt::try_from(max_supply.as_slice()).unwrap(),
        AccountStorageMode::Private,
        auth_scheme,
    )
    .unwrap();

    client
        .insert_account(&faucet, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();

    let asset: miden_objects::assets::Asset = FungibleAsset::new(faucet.id(), 5u64).unwrap().into();
    let payment_data = PaymentTransactionData::new(asset, sender_account.id(), target_account.id());
    // Insert a P2ID transaction object

    let transaction_request =
        TransactionRequest::pay_to_id(payment_data, None, NoteType::Private, client.rng()).unwrap();
    let transaction_execution_result =
        client.new_transaction(sender_account.id(), transaction_request).unwrap();

    client.submit_transaction(transaction_execution_result).await.unwrap();
}

pub fn mock_fungible_faucet_account(
    id: AccountId,
    initial_balance: u64,
    key_pair: SecretKey,
) -> Account {
    let mut rng = rand::thread_rng();
    let init_seed: [u8; 32] = rng.gen();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key: key_pair.public_key() };

    let (faucet, _seed) = miden_lib::accounts::faucets::create_basic_fungible_faucet(
        init_seed,
        TokenSymbol::new("TST").unwrap(),
        10u8,
        Felt::try_from(initial_balance.to_le_bytes().as_slice())
            .expect("u64 can be safely converted to a field element"),
        AccountStorageMode::Private,
        auth_scheme,
    )
    .unwrap();

    let faucet_storage_slot_1 =
        [Felt::new(initial_balance), Felt::new(0), Felt::new(0), Felt::new(0)];
    let faucet_account_storage = AccountStorage::new(
        vec![
            SlotItem {
                index: 0,
                slot: StorageSlot::new_value(key_pair.public_key().into()),
            },
            SlotItem {
                index: 1,
                slot: StorageSlot::new_value(faucet_storage_slot_1),
            },
        ],
        BTreeMap::new(),
    )
    .unwrap();

    Account::from_parts(
        id,
        AssetVault::new(&[]).unwrap(),
        faucet_account_storage.clone(),
        faucet.code().clone(),
        Felt::new(10u64),
    )
}

pub fn mock_notes(assembler: Assembler) -> (Vec<Note>, Vec<Note>) {
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
    let note_script = NoteScript::compile("begin push.1 drop end", assembler.clone()).unwrap();

    let note_tag: NoteTag =
        NoteTag::from_account_id(sender, miden_objects::notes::NoteExecutionMode::Local).unwrap();

    // Created Notes
    const SERIAL_NUM_4: Word = [Felt::new(13), Felt::new(14), Felt::new(15), Felt::new(16)];
    let note_metadata = NoteMetadata::new(
        sender,
        NoteType::Private,
        note_tag,
        NoteExecutionHint::None,
        Default::default(),
    )
    .unwrap();
    let note_assets = NoteAssets::new(vec![fungible_asset_1]).unwrap();
    let note_recipient =
        NoteRecipient::new(SERIAL_NUM_4, note_script.clone(), NoteInputs::new(vec![]).unwrap());

    let created_note_1 = Note::new(note_assets, note_metadata, note_recipient);

    const SERIAL_NUM_5: Word = [Felt::new(17), Felt::new(18), Felt::new(19), Felt::new(20)];
    let note_metadata = NoteMetadata::new(
        sender,
        NoteType::Private,
        note_tag,
        NoteExecutionHint::None,
        Default::default(),
    )
    .unwrap();
    let note_recipient =
        NoteRecipient::new(SERIAL_NUM_5, note_script.clone(), NoteInputs::new(vec![]).unwrap());
    let note_assets = NoteAssets::new(vec![fungible_asset_2]).unwrap();
    let created_note_2 = Note::new(note_assets, note_metadata, note_recipient);

    const SERIAL_NUM_6: Word = [Felt::new(21), Felt::new(22), Felt::new(23), Felt::new(24)];
    let note_metadata = NoteMetadata::new(
        sender,
        NoteType::Private,
        note_tag,
        NoteExecutionHint::None,
        Default::default(),
    )
    .unwrap();
    let note_assets = NoteAssets::new(vec![fungible_asset_3]).unwrap();
    let note_recipient =
        NoteRecipient::new(SERIAL_NUM_6, note_script, NoteInputs::new(vec![Felt::new(2)]).unwrap());
    let created_note_3 = Note::new(note_assets, note_metadata, note_recipient);

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
        created_note_0_recipient = prepare_word(&created_notes[0].recipient().digest()),
        created_note_0_tag = created_notes[0].metadata().tag(),
        created_note_0_asset = prepare_assets(created_notes[0].assets())[0],
        created_note_1_recipient = prepare_word(&created_notes[1].recipient().digest()),
        created_note_1_tag = created_notes[1].metadata().tag(),
        created_note_1_asset = prepare_assets(created_notes[1].assets())[0],
    );
    let _note_1_script = NoteScript::compile(note_1_script_src, assembler.clone()).unwrap();

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
        created_note_2_recipient = prepare_word(&created_notes[2].recipient().digest()),
        created_note_2_tag = created_notes[2].metadata().tag(),
        created_note_2_asset = prepare_assets(created_notes[2].assets())[0],
    );
    let note_2_script = NoteScript::compile(note_2_script_src, assembler.clone()).unwrap();

    // Consumed Notes
    const SERIAL_NUM_1: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let note_metadata = NoteMetadata::new(
        sender,
        NoteType::Private,
        note_tag,
        NoteExecutionHint::None,
        Default::default(),
    )
    .unwrap();
    let note_recipient = NoteRecipient::new(
        SERIAL_NUM_1,
        note_2_script.clone(),
        NoteInputs::new(vec![Felt::new(1)]).unwrap(),
    );
    let note_assets = NoteAssets::new(vec![fungible_asset_1]).unwrap();
    let consumed_note_1 = Note::new(note_assets, note_metadata, note_recipient);

    const SERIAL_NUM_2: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
    let note_metadata = NoteMetadata::new(
        sender,
        NoteType::Private,
        note_tag,
        NoteExecutionHint::None,
        Default::default(),
    )
    .unwrap();
    let note_assets = NoteAssets::new(vec![fungible_asset_2, fungible_asset_3]).unwrap();
    let note_recipient = NoteRecipient::new(
        SERIAL_NUM_2,
        note_2_script,
        NoteInputs::new(vec![Felt::new(2)]).unwrap(),
    );

    let consumed_note_2 = Note::new(note_assets, note_metadata, note_recipient);

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
    let account_assembler = TransactionKernel::assembler();

    let account_code = AccountCode::compile(account_code_src, account_assembler).unwrap();
    let slot_item = SlotItem {
        index: 0,
        slot: StorageSlot::new_value(public_key),
    };
    let account_storage = AccountStorage::new(vec![slot_item], BTreeMap::new()).unwrap();

    let asset_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::from_parts(account_id, asset_vault, account_storage, account_code, Felt::new(nonce))
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

pub fn create_test_client() -> MockClient {
    let store: SqliteStoreConfig = create_test_store_path()
        .into_os_string()
        .into_string()
        .unwrap()
        .try_into()
        .unwrap();

    let rpc_config = RpcConfig::default();
    let rpc_endpoint = rpc_config.endpoint.to_string();

    let store = SqliteStore::new(&store).unwrap();
    let store = Rc::new(store);

    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));

    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);

    MockClient::new(MockRpcApi::new(&rpc_endpoint), rng, store, authenticator, true)
}

pub fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}
