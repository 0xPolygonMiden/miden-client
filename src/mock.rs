use crate::client::transactions::{PaymentTransactionData, TransactionTemplate};
use crate::client::{Client, FILTER_ID_SHIFT};
use crate::store::mock_executor_data_store::MockDataStore;
use crypto::{dsa::rpo_falcon512::KeyPair, StarkField};
use crypto::{Felt, FieldElement};
use miden_lib::transaction::TransactionKernel;
use miden_node_proto::block_header::BlockHeader as NodeBlockHeader;
use miden_node_proto::merkle::MerklePath;
use miden_node_proto::note::NoteSyncRecord;
use miden_node_proto::requests::SubmitProvenTransactionRequest;
use miden_node_proto::responses::SubmitProvenTransactionResponse;
use miden_node_proto::{
    account::AccountId as ProtoAccountId,
    requests::SyncStateRequest,
    responses::{NullifierUpdate, SyncStateResponse},
};
use mock::constants::{generate_account_seed, AccountSeedType};
use mock::mock::account::mock_account;
use mock::mock::block;
use mock::mock::notes::mock_notes;
use objects::transaction::ChainMmr;
use objects::utils::collections::BTreeMap;
use objects::BlockHeader;

use crate::store::accounts::AuthInfo;

use crate::mock::block::mock_block_header;
use miden_tx::TransactionExecutor;
use objects::accounts::{AccountId, AccountType};
use objects::assets::FungibleAsset;
use objects::crypto::merkle::Mmr;
use objects::crypto::merkle::MmrDelta;
use objects::crypto::merkle::NodeIndex;
use objects::crypto::merkle::PartialMmr;
use objects::crypto::merkle::SimpleSmt;
use objects::notes::Note;
use objects::notes::NoteInclusionProof;
use objects::notes::NOTE_LEAF_DEPTH;
use objects::notes::NOTE_TREE_DEPTH;
use objects::transaction::InputNote;

/// Mock RPC API
///
/// This struct implements the RPC API used by the client to communicate with the node. It is
/// intended to be used for testing purposes only.
pub struct MockRpcApi {
    pub sync_state_requests: BTreeMap<SyncStateRequest, SyncStateResponse>,
}

impl Default for MockRpcApi {
    fn default() -> Self {
        Self {
            sync_state_requests: generate_sync_state_mock_requests(),
        }
    }
}

impl MockRpcApi {
    /// Executes the specified sync state request and returns the response.
    pub async fn sync_state(
        &mut self,
        request: impl tonic::IntoRequest<SyncStateRequest>,
    ) -> std::result::Result<tonic::Response<SyncStateResponse>, tonic::Status> {
        let request: SyncStateRequest = request.into_request().into_inner();
        match self
            .sync_state_requests
            .iter()
            .find(|(req, _resp)| req.block_num == request.block_num)
        {
            Some((_req, response)) => {
                let response = response.clone();
                Ok(tonic::Response::new(response))
            }
            None => Err(tonic::Status::not_found(
                "no response for sync state request",
            )),
        }
    }

    pub async fn submit_proven_transaction(
        &mut self,
        request: impl tonic::IntoRequest<SubmitProvenTransactionRequest>,
    ) -> std::result::Result<tonic::Response<SubmitProvenTransactionResponse>, tonic::Status> {
        let _request = request.into_request().into_inner();
        let response = SubmitProvenTransactionResponse {};

        Ok(tonic::Response::new(response))
    }
}

fn create_mock_two_step_sync_state_request(
    requests: &mut BTreeMap<SyncStateRequest, SyncStateResponse>,
    account_id: AccountId,
    recorded_notes: &[InputNote],
    mmr_delta: Option<MmrDelta>,
    last_block_header: Option<BlockHeader>,
) {
    use mock::mock::notes::AssetPreservationStatus;

    // Clear existing mocked data
    requests.clear();

    let accounts = vec![ProtoAccountId {
        id: u64::from(account_id),
    }];

    let nullifiers: Vec<u32> = recorded_notes
        .iter()
        .map(|note| (note.note().nullifier().as_elements()[3].as_int() >> FILTER_ID_SHIFT) as u32)
        .collect();

    let assembler = TransactionKernel::assembler();
    let account = mock_account(None, Felt::ONE, None, &assembler);
    let (_consumed, created_notes) = mock_notes(&assembler, &AssetPreservationStatus::Preserved);

    // create a state sync request / response pair for the scenario where there is an needed update
    // 2 blocks before the current chain tip
    let request = SyncStateRequest {
        block_num: 0,
        account_ids: accounts.clone(),
        note_tags: vec![],
        nullifiers: nullifiers.clone(),
    };

    let block_header: objects::BlockHeader =
        last_block_header.unwrap_or(block::mock_block_header(10, None, None, &[]));
    let chain_tip = block_header.block_num();

    // create a block header for the response
    let prior_block_header: objects::BlockHeader =
        block::mock_block_header(chain_tip - 2, None, None, &[]);

    // create a state sync response
    let response = SyncStateResponse {
        chain_tip,
        mmr_delta: None,
        block_path: None,
        block_header: Some(NodeBlockHeader::from(prior_block_header)),
        accounts: vec![],
        notes: vec![NoteSyncRecord {
            note_index: 0,
            note_hash: Some(created_notes.first().unwrap().authentication_hash().into()),
            sender: account.id().into(),
            tag: 0u64,
            num_assets: 2,
            merkle_path: Some(MerklePath::default()),
        }],
        nullifiers: vec![NullifierUpdate {
            nullifier: Some(
                recorded_notes
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

    // SECOND REQUEST
    // ---------------------------------------------------------------------------------

    // create a state sync request
    let request = SyncStateRequest {
        block_num: prior_block_header.block_num(),
        account_ids: accounts.clone(),
        note_tags: vec![],
        nullifiers,
    };

    // create a block header for the response
    let block_header: objects::BlockHeader =
        last_block_header.unwrap_or(block::mock_block_header(chain_tip, None, None, &[]));

    // create a state sync response
    let response = SyncStateResponse {
        chain_tip,
        mmr_delta: mmr_delta.map(|inner_delta| inner_delta.into()),
        block_path: None,
        block_header: Some(NodeBlockHeader::from(block_header)),
        accounts: vec![],
        notes: vec![NoteSyncRecord {
            note_index: 0,
            note_hash: Some(created_notes.first().unwrap().authentication_hash().into()),
            sender: account.id().into(),
            tag: 0u64,
            num_assets: 2,
            merkle_path: Some(MerklePath::default()),
        }],
        nullifiers: vec![NullifierUpdate {
            nullifier: Some(
                recorded_notes
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

/// Generates mock sync state requests and responses
fn generate_sync_state_mock_requests() -> BTreeMap<SyncStateRequest, SyncStateResponse> {
    use mock::mock::{
        account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs,
    };

    // generate test data
    let transaction_inputs = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    // create sync state requests
    let mut requests = BTreeMap::new();

    create_mock_two_step_sync_state_request(
        &mut requests,
        transaction_inputs.account().id(),
        &transaction_inputs.input_notes().clone().into_vec(),
        None,
        None,
    );

    requests
}

fn mock_full_chain_mmr_and_notes(consumed_notes: Vec<Note>) -> (Mmr, Vec<InputNote>, BlockHeader) {
    let mut note_trees = Vec::new();

    // TODO: Consider how to better represent note authentication data.
    // we use the index for both the block number and the leaf index in the note tree
    for (index, note) in consumed_notes.iter().enumerate() {
        let tree_index = 2 * index;
        let smt_entries = vec![
            (tree_index as u64, note.id().into()),
            ((tree_index + 1) as u64, note.metadata().into()),
        ];
        let smt = SimpleSmt::with_leaves(NOTE_LEAF_DEPTH, smt_entries).unwrap();
        note_trees.push(smt);
    }

    let mut note_tree_iter = note_trees.iter();

    // create a dummy chain of block headers
    let block_chain = vec![
        mock_block_header(0, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(1, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(2, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(3, None, note_tree_iter.next().map(|x| x.root()), &[]),
        mock_block_header(4, None, note_tree_iter.next().map(|x| x.root()), &[]),
    ];

    // instantiate and populate MMR
    let mut mmr = Mmr::default();
    for block_header in block_chain.iter() {
        mmr.add(block_header.hash())
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
                    note_trees[index].get_path(auth_index).unwrap(),
                )
                .unwrap(),
            )
        })
        .collect::<Vec<_>>();

    (mmr, recorded_notes, block_chain[block_chain.len() - 1])
}

/// inserts mock note and account data into the client, returns the latest block header and the
/// chain mmr
pub fn insert_mock_data(client: &mut Client) -> (BlockHeader, ChainMmr) {
    use mock::mock::notes::AssetPreservationStatus;

    // mock notes
    let assembler = TransactionKernel::assembler();
    let (account_id, account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);
    let account = mock_account(Some(u64::from(account_id)), Felt::ONE, None, &assembler);
    let (input_notes, created_notes) = mock_notes(&assembler, &AssetPreservationStatus::Preserved);

    let (mmr, recorded_notes, last_block_header) = mock_full_chain_mmr_and_notes(input_notes);

    // insert notes into database
    for note in recorded_notes.clone() {
        client.import_input_note(note.into()).unwrap();
    }

    // insert notes into database
    for note in created_notes {
        client.import_input_note(note.into()).unwrap();
    }

    // insert account
    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();
    client
        .insert_account(&account, account_seed, &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();

    // Create the Mmr delta update
    let mmr_delta = mmr.get_delta(0, mmr.forest());

    create_mock_two_step_sync_state_request(
        &mut client.rpc_api.sync_state_requests,
        account.id(),
        &recorded_notes,
        mmr_delta.ok(),
        Some(last_block_header),
    );

    (last_block_header, mmr_to_chain_mmr(&mmr))
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

impl Client {
    /// testing function to set a data store to conveniently mock data if needed
    pub fn set_data_store(&mut self, data_store: MockDataStore) {
        self.tx_executor = TransactionExecutor::new(data_store);
    }
}

/// Helper Functions

/// Converts the MMR into partial MMR by copying all leaves from MMR to partial MMR. Taken from
/// mock crate
pub fn mmr_to_chain_mmr(mmr: &Mmr) -> ChainMmr {
    let num_leaves = mmr.forest();
    let mut partial_mmr = PartialMmr::from_peaks(mmr.peaks(mmr.forest()).unwrap());
    let mut headers = BTreeMap::new();

    for i in 0..num_leaves {
        let node = mmr.get(i).unwrap();
        let path = mmr.open(i, mmr.forest()).unwrap().merkle_path;
        partial_mmr.add(i, node, &path).unwrap();
        headers.insert(i as u32, node);
    }

    ChainMmr::new(partial_mmr, headers).unwrap()
}
