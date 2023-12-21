use crate::client::transactions::{PaymentTransactionData, TransactionTemplate};
use crate::client::{Client, FILTER_ID_SHIFT};
use crate::store::mock_executor_data_store::MockDataStore;
use crypto::{dsa::rpo_falcon512::KeyPair, StarkField};
use miden_node_proto::block_header::BlockHeader as NodeBlockHeader;
use miden_node_proto::requests::SubmitProvenTransactionRequest;
use miden_node_proto::responses::SubmitProvenTransactionResponse;
use miden_node_proto::{
    account_id::AccountId as ProtoAccountId,
    requests::SyncStateRequest,
    responses::{NullifierUpdate, SyncStateResponse},
};
use mock::mock::block;
use objects::utils::collections::BTreeMap;

use crate::store::accounts::AuthInfo;

use miden_tx::TransactionExecutor;
use objects::accounts::AccountType;
use objects::assets::FungibleAsset;

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
        let request = request.into_request().into_inner();
        match self.sync_state_requests.get(&request) {
            Some(response) => {
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

/// Generates mock sync state requests and responses
fn generate_sync_state_mock_requests() -> BTreeMap<SyncStateRequest, SyncStateResponse> {
    use mock::mock::{
        account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs,
    };

    // generate test data
    let (account, _, _, recorded_notes, _) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    let accounts = vec![ProtoAccountId {
        id: u64::from(account.id()),
    }];

    let nullifiers = recorded_notes
        .iter()
        .map(|note| (note.note().nullifier().as_elements()[3].as_int() >> FILTER_ID_SHIFT) as u32)
        .collect();

    // create sync state requests
    let mut requests = BTreeMap::new();

    // create a state sync request
    let request = SyncStateRequest {
        block_num: 0,
        account_ids: accounts,
        note_tags: vec![],
        nullifiers,
    };

    let chain_tip = 10;

    // create a block header for the response
    let block_header: objects::BlockHeader = block::mock_block_header(chain_tip, None, None, &[]);

    // create a state sync response
    let response = SyncStateResponse {
        chain_tip,
        mmr_delta: None,
        block_path: None,
        block_header: Some(NodeBlockHeader::from(block_header)),
        accounts: vec![],
        notes: vec![],
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

    requests
}

/// inserts mock note and account data into the client
pub fn insert_mock_data(client: &mut Client) {
    use mock::mock::{
        account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs,
    };

    // generate test data
    let (account, _, _, recorded_notes, _) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    // insert notes into database
    for note in recorded_notes.into_iter() {
        client.insert_input_note(note).unwrap();
    }

    // insert account
    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();
    client
        .insert_account(&account, &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();
}

pub async fn create_mock_transaction(client: &mut Client) {
    let key_pair: KeyPair = KeyPair::new()
        .map_err(|err| format!("Error generating KeyPair: {}", err))
        .unwrap();
    let auth_scheme: miden_lib::AuthScheme = miden_lib::AuthScheme::RpoFalcon512 {
        pub_key: key_pair.public_key(),
    };
    let _assembler = miden_lib::assembler::assembler();

    let mut rng = rand::thread_rng();
    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = rand::Rng::gen(&mut rng);

    let (sender_account, _) = miden_lib::wallets::create_basic_wallet(
        init_seed,
        auth_scheme,
        AccountType::RegularAccountImmutableCode,
    )
    .unwrap();

    client
        .insert_account(&sender_account, &AuthInfo::RpoFalcon512(key_pair))
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

    let (target_account, _) = miden_lib::wallets::create_basic_wallet(
        init_seed,
        auth_scheme,
        AccountType::RegularAccountImmutableCode,
    )
    .unwrap();

    client
        .insert_account(&target_account, &AuthInfo::RpoFalcon512(key_pair))
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

    let (faucet, _) = miden_lib::faucets::create_basic_fungible_faucet(
        init_seed,
        objects::assets::TokenSymbol::new("MOCK").unwrap(),
        4u8,
        crypto::Felt::try_from(max_supply.as_slice()).unwrap(),
        auth_scheme,
    )
    .unwrap();

    client
        .insert_account(&faucet, &AuthInfo::RpoFalcon512(key_pair))
        .unwrap();

    let asset: objects::assets::Asset = FungibleAsset::new(faucet.id(), 5u64).unwrap().into();
    let transaction_template = TransactionTemplate::PayToId(PaymentTransactionData::new(
        asset,
        sender_account.id(),
        target_account.id(),
    ));
    let (transaction_result, script) = client.new_transaction(transaction_template).unwrap();

    client
        .send_transaction(transaction_result.into_witness(), Some(script))
        .await
        .unwrap();
}

#[cfg(any(test, feature = "testing"))]
impl Client {
    /// testing function to set a data store to conveniently mock data if needed
    pub fn set_data_store(&mut self, data_store: MockDataStore) {
        self.tx_executor = TransactionExecutor::new(data_store);
    }
}
