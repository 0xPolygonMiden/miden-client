use crate::{
    client::{
        sync_state::FILTER_ID_SHIFT,
        transactions::{PaymentTransactionData, TransactionTemplate},
        Client, RpcApiEndpoint,
    },
    errors::RpcApiError,
};
use crypto::{dsa::rpo_falcon512::KeyPair, Felt, FieldElement, StarkField};
use miden_lib::transaction::TransactionKernel;
use miden_node_proto::{
    account::AccountId as ProtoAccountId,
    block_header::BlockHeader as NodeBlockHeader,
    merkle::MerklePath,
    note::NoteSyncRecord,
    requests::{SubmitProvenTransactionRequest, SyncStateRequest},
    responses::{NullifierUpdate, SubmitProvenTransactionResponse, SyncStateResponse},
};
use mock::{
    constants::{generate_account_seed, AccountSeedType},
    mock::account::mock_account,
};

use mock::mock::{
    block,
    notes::{mock_notes, AssetPreservationStatus},
};
use objects::{transaction::InputNotes, utils::collections::BTreeMap};

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
    ) -> Result<tonic::Response<SyncStateResponse>, RpcApiError> {
        let request: SyncStateRequest = request.into_request().into_inner();

        // Match request -> response through block_nu,
        match self
            .sync_state_requests
            .iter()
            .find(|(req, _resp)| req.block_num == request.block_num)
        {
            Some((_req, response)) => {
                let response = response.clone();
                Ok(tonic::Response::new(response))
            }
            None => Err(RpcApiError::RequestError(
                RpcApiEndpoint::SyncState,
                tonic::Status::not_found("no response for sync state request"),
            )),
        }
    }

    pub async fn submit_proven_transaction(
        &mut self,
        request: impl tonic::IntoRequest<SubmitProvenTransactionRequest>,
    ) -> Result<tonic::Response<SubmitProvenTransactionResponse>, RpcApiError> {
        let _request = request.into_request().into_inner();
        let response = SubmitProvenTransactionResponse {};

        // TODO: add some basic validations to test error cases

        Ok(tonic::Response::new(response))
    }
}

/// Generates mock sync state requests and responses
fn create_mock_sync_state_request_for_account_and_notes(
    requests: &mut BTreeMap<SyncStateRequest, SyncStateResponse>,
    account_id: AccountId,
    recorded_notes: &InputNotes,
) {
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

    // create a state sync request
    let request = SyncStateRequest {
        block_num: 0,
        account_ids: accounts.clone(),
        note_tags: vec![],
        nullifiers: nullifiers.clone(),
    };

    let chain_tip = 10;

    // create a block header for the response
    let block_header: objects::BlockHeader = block::mock_block_header(8, None, None, &[]);

    // create a state sync response
    let response = SyncStateResponse {
        chain_tip,
        mmr_delta: None,
        block_path: None,
        block_header: Some(NodeBlockHeader::from(block_header)),
        accounts: vec![],
        notes: vec![NoteSyncRecord {
            note_index: 0,
            note_hash: Some(created_notes.first().unwrap().id().into()),
            sender: account.id().into(),
            tag: 0u64,
            num_assets: 2,
            merkle_path: Some(MerklePath::default()),
        }],
        nullifiers: vec![NullifierUpdate {
            nullifier: Some(recorded_notes.get_note(0).note().nullifier().inner().into()),
            block_num: 7,
        }],
    };
    requests.insert(request, response);

    // SECOND REQUEST
    // ---------------------------------------------------------------------------------

    // create a state sync request
    let request = SyncStateRequest {
        block_num: 8,
        account_ids: accounts,
        note_tags: vec![],
        nullifiers,
    };

    // create a block header for the response
    let block_header: objects::BlockHeader = block::mock_block_header(10, None, None, &[]);

    // create a state sync response
    let response = SyncStateResponse {
        chain_tip,
        mmr_delta: None,
        block_path: None,
        block_header: Some(NodeBlockHeader::from(block_header)),
        accounts: vec![],
        notes: vec![NoteSyncRecord {
            note_index: 0,
            note_hash: Some(created_notes.first().unwrap().id().into()),
            sender: account.id().into(),
            tag: 0u64,
            num_assets: 2,
            merkle_path: Some(MerklePath::default()),
        }],
        nullifiers: vec![NullifierUpdate {
            nullifier: Some(recorded_notes.get_note(0).note().nullifier().inner().into()),
            block_num: 7,
        }],
    };

    requests.insert(request, response);
}

/// Generates mock sync state requests and responses
fn generate_sync_state_mock_requests() -> BTreeMap<SyncStateRequest, SyncStateResponse> {
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
        transaction_inputs.input_notes(),
    );

    requests
}

/// inserts mock note and account data into the client
pub async fn insert_mock_data(client: &mut Client) {
    use mock::mock::{account::MockAccountType, transaction::mock_inputs};

    // generate test data
    let transaction_inputs = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    let assembler = TransactionKernel::assembler();
    let (account_id, account_seed) =
        generate_account_seed(AccountSeedType::RegularAccountUpdatableCodeOnChain);
    let account = mock_account(Some(account_id.into()), Felt::ONE, None, &assembler);
    let (_consumed, created_notes) = mock_notes(&assembler, &AssetPreservationStatus::Preserved);

    // insert notes into database
    for note in transaction_inputs.input_notes().clone().into_iter() {
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

    // insert some sync request
    create_mock_sync_state_request_for_account_and_notes(
        &mut client.rpc_api.sync_state_requests,
        account_id,
        transaction_inputs.input_notes(),
    );
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
