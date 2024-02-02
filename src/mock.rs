use crate::{
    client::{
        rpc_client::StateSyncInfo,
        sync::FILTER_ID_SHIFT,
        transactions::{PaymentTransactionData, TransactionTemplate},
        Client, RpcApiEndpoint,
    },
    errors::RpcApiError,
};
use crypto::{
    dsa::rpo_falcon512::KeyPair,
    merkle::{NodeIndex, SimpleSmt},
    Felt, FieldElement, StarkField,
};
use miden_lib::transaction::TransactionKernel;
use miden_node_proto::{
    account::AccountId as ProtoAccountId,
    block_header::BlockHeader as NodeBlockHeader,
    note::NoteSyncRecord,
    requests::{GetBlockHeaderByNumberRequest, SubmitProvenTransactionRequest, SyncStateRequest},
    responses::{NullifierUpdate, SubmitProvenTransactionResponse, SyncStateResponse},
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
    crypto::merkle::{Mmr, MmrDelta},
    notes::{Note, NoteInclusionProof},
    transaction::InputNote,
    utils::collections::BTreeMap,
    BlockHeader, NOTE_TREE_DEPTH,
};
use tonic::{IntoRequest, Response, Status};

use crate::store::accounts::AuthInfo;

use objects::{
    accounts::{AccountId, AccountType},
    assets::FungibleAsset,
};

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
    /// Executes the specified sync state request and returns the response.
    pub async fn sync_state(
        &mut self,
        block_num: u32,
        _account_ids: &[AccountId],
        _note_tags: &[u16],
        _nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, RpcApiError> {
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
            None => Err(RpcApiError::RequestError(
                RpcApiEndpoint::SyncState,
                Status::not_found("no response for sync state request"),
            )),
        }?;

        response.into_inner().try_into()
    }

    /// Creates and executes a [GetBlockHeaderByNumberRequest].
    /// Only used for retrieving genesis block right now so that's the only case we need to cover.
    pub async fn get_block_header_by_number(
        &mut self,
        request: impl IntoRequest<GetBlockHeaderByNumberRequest>,
    ) -> Result<BlockHeader, RpcApiError> {
        let request: GetBlockHeaderByNumberRequest = request.into_request().into_inner();

        if request.block_num == Some(0) {
            let block_header: objects::BlockHeader = block::mock_block_header(0, None, None, &[]);
            return Ok(block_header);
        }
        panic!("get_block_header_by_number is supposed to be only used for genesis block")
    }

    pub async fn submit_proven_transaction(
        &mut self,
        request: impl tonic::IntoRequest<SubmitProvenTransactionRequest>,
    ) -> std::result::Result<tonic::Response<SubmitProvenTransactionResponse>, RpcApiError> {
        let _request = request.into_request().into_inner();
        let response = SubmitProvenTransactionResponse {};

        // TODO: add some basic validations to test error cases

        Ok(Response::new(response))
    }
}

/// Generates mock sync state requests and responses
fn create_mock_sync_state_request_for_account_and_notes(
    requests: &mut BTreeMap<SyncStateRequest, SyncStateResponse>,
    account_id: AccountId,
    created_notes: &[Note],
    consumed_notes: &[InputNote],
    mmr_delta: Option<Vec<MmrDelta>>,
    tracked_block_headers: Option<Vec<BlockHeader>>,
) {
    // Clear existing mocked data
    requests.clear();

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
                .map(miden_node_proto::mmr::MmrDelta::from),
            block_header: Some(NodeBlockHeader::from(*block_header)),
            accounts: vec![],
            notes: vec![NoteSyncRecord {
                note_index: 0,
                note_hash: Some(created_notes_iter.next().unwrap().id().into()),
                sender: account.id().into(),
                tag: 0u64,
                merkle_path: Some(miden_node_proto::merkle::MerklePath::default()),
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
    let mut requests = BTreeMap::new();

    create_mock_sync_state_request_for_account_and_notes(
        &mut requests,
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
pub async fn insert_mock_data(client: &mut Client) -> Vec<BlockHeader> {
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

    create_mock_sync_state_request_for_account_and_notes(
        &mut client.rpc_api.state_sync_requests,
        account.id(),
        &created_notes,
        &consumed_notes,
        Some(mmr_deltas),
        Some(tracked_block_headers.clone()),
    );

    tracked_block_headers
}

pub async fn create_mock_transaction(client: &mut Client) {
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

#[cfg(test)]
impl Client {
    /// Helper function to set a data store to conveniently mock data for tests
    pub fn set_data_store(
        &mut self,
        data_store: crate::store::mock_executor_data_store::MockDataStore,
    ) {
        self.tx_executor = miden_tx::TransactionExecutor::new(data_store);
    }
}
