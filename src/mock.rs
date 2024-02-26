#[cfg(test)]
use crate::store::Store;
use crate::{
    client::{
        rpc::{NodeRpcClient, NodeRpcClientEndpoint, StateSyncInfo},
        sync::FILTER_ID_SHIFT,
        transactions::{PaymentTransactionData, TransactionTemplate},
        Client,
    },
    errors::NodeRpcClientError,
    store::{sqlite_store::SqliteStore, AuthInfo},
};
use async_trait::async_trait;
use crypto::{
    dsa::rpo_falcon512::KeyPair,
    merkle::{NodeIndex, SimpleSmt},
    Felt, FieldElement,
};
use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_node_proto::generated::{
    account::AccountId as ProtoAccountId,
    block_header::BlockHeader as NodeBlockHeader,
    note::NoteSyncRecord,
    requests::{GetBlockHeaderByNumberRequest, SyncStateRequest},
    responses::{NullifierUpdate, SyncStateResponse},
};

use mock::{
    constants::{generate_account_seed, AccountSeedType},
    mock::{account::mock_account, block::mock_block_header},
};

use mock::mock::{
    block,
    notes::{mock_notes, AssetPreservationStatus},
};
use objects::{
    accounts::{Account, AccountStorage, StorageSlotType},
    accounts::{AccountId, AccountType},
    assets::FungibleAsset,
    assets::{AssetVault, TokenSymbol},
    crypto::merkle::{Mmr, MmrDelta},
    notes::{Note, NoteInclusionProof},
    transaction::{InputNote, ProvenTransaction},
    utils::collections::BTreeMap,
    BlockHeader, NOTE_TREE_DEPTH,
};
use rand::Rng;
use tonic::{IntoRequest, Response, Status};

pub use crate::store::mock_executor_data_store::MockDataStore;

pub type MockClient = Client<MockRpcApi, SqliteStore>;

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
        let response = match self
            .state_sync_requests
            .iter()
            .find(|(req, _)| req.block_num == block_num)
        {
            Some((_req, response)) => {
                let response = response.clone();
                Ok(Response::new(response))
            }
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
            let block_header: objects::BlockHeader = block::mock_block_header(0, None, None, &[]);
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

/// Generates mock sync state requests and responses
fn create_mock_sync_state_request_for_account_and_notes(
    account_id: AccountId,
    created_notes: &[Note],
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

    let assembler = TransactionKernel::assembler();
    let account = mock_account(None, Felt::ONE, None, &assembler);

    let tracked_block_headers = tracked_block_headers.unwrap_or(vec![
        block::mock_block_header(8, None, None, &[]),
        block::mock_block_header(10, None, None, &[]),
    ]);

    let chain_tip = tracked_block_headers
        .last()
        .map(|header| header.block_num())
        .unwrap_or(10);
    let mut deltas_iter = mmr_delta.unwrap_or_default().into_iter();
    let mut created_notes_iter = created_notes.iter();

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
            mmr_delta: deltas_iter
                .next()
                .map(miden_node_proto::generated::mmr::MmrDelta::from),
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
                nullifier: Some(
                    consumed_notes
                        .first()
                        .unwrap()
                        .note()
                        .nullifier()
                        .inner()
                        .into(),
                ),
                block_num: 7,
            }],
        };
        requests.insert(request, response);
    }

    requests
}

/// Generates mock sync state requests and responses
fn generate_state_sync_mock_requests() -> BTreeMap<SyncStateRequest, SyncStateResponse> {
    use mock::mock::{account::MockAccountType, transaction::mock_inputs};

    // generate test data
    let transaction_inputs = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    // create sync state requests
    let requests = create_mock_sync_state_request_for_account_and_notes(
        transaction_inputs.account().id(),
        &transaction_inputs
            .input_notes()
            .clone()
            .into_iter()
            .map(|input_note| input_note.note().clone())
            .collect::<Vec<_>>(),
        &transaction_inputs.input_notes().clone().into_vec(),
        None,
        None,
    );

    requests
}

fn mock_full_chain_mmr_and_notes(
    consumed_notes: Vec<Note>,
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
        mock_block_header(0, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(1, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(2, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(3, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(4, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(5, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(6, None, note_tree_iter.next().map(|x| x.root()), &[]),
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
    let assembler = TransactionKernel::assembler();
    let (account_id, account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);
    let account = mock_account(Some(u64::from(account_id)), Felt::ONE, None, &assembler);
    let (input_notes, created_notes) = mock_notes(&assembler, &AssetPreservationStatus::Preserved);

    let (_mmr, consumed_notes, tracked_block_headers, mmr_deltas) =
        mock_full_chain_mmr_and_notes(input_notes);

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
        .insert_account(&account, account_seed, &AuthInfo::RpoFalcon512(key_pair))
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
        .insert_account(&sender_account, seed, &AuthInfo::RpoFalcon512(key_pair))
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
        .insert_account(&target_account, seed, &AuthInfo::RpoFalcon512(key_pair))
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
        objects::assets::TokenSymbol::new("MOCK").unwrap(),
        4u8,
        crypto::Felt::try_from(max_supply.as_slice()).unwrap(),
        auth_scheme,
    )
    .unwrap();

    client
        .insert_account(&faucet, seed, &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();

    let asset: objects::assets::Asset = FungibleAsset::new(faucet.id(), 5u64).unwrap().into();

    // Insert a P2ID transaction object

    let transaction_template = TransactionTemplate::PayToId(PaymentTransactionData::new(
        asset,
        sender_account.id(),
        target_account.id(),
    ));

    let transaction_execution_result = client.new_transaction(transaction_template).unwrap();

    client
        .send_transaction(transaction_execution_result)
        .await
        .unwrap();
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

    let faucet_storage_slot_1 = [
        Felt::new(initial_balance),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ];
    let faucet_account_storage = AccountStorage::new(vec![
        (
            0,
            (
                StorageSlotType::Value { value_arity: 0 },
                key_pair.public_key().into(),
            ),
        ),
        (
            1,
            (
                StorageSlotType::Value { value_arity: 0 },
                faucet_storage_slot_1,
            ),
        ),
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
    pub fn set_data_store(&mut self, data_store: MockDataStore) {
        self.set_tx_executor(miden_tx::TransactionExecutor::new(data_store));
    }
}
