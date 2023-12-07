use crate::store::AuthInfo;

use super::Client;
use super::FILTER_ID_SHIFT;
use crypto::dsa::rpo_falcon512::KeyPair;
use miden_node_proto::requests::SubmitProvenTransactionRequest;
use miden_node_proto::responses::SubmitProvenTransactionResponse;
use miden_node_proto::{
    account_id::AccountId as ProtoAccountId,
    requests::SyncStateRequest,
    responses::{NullifierUpdate, SyncStateResponse},
};
use miden_tx::DataStore;
use mock::mock::transaction::mock_executed_tx;
use objects::{utils::collections::BTreeMap, StarkField};

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
        let response = self
            .sync_state_requests
            .get(&request)
            .expect("no response for sync state request")
            .clone();
        Ok(tonic::Response::new(response))
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
    let (account, _, _, recorded_notes) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    let accounts = vec![ProtoAccountId {
        id: u64::from(account.id()),
    }];

    let nullifiers = recorded_notes
        .iter()
        .map(|note| (note.note().nullifier()[3].as_int() >> FILTER_ID_SHIFT) as u32)
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

    // create a state sync response
    let response = SyncStateResponse {
        chain_tip: 10,
        mmr_delta: None,
        block_path: None,
        block_header: None,
        accounts: vec![],
        notes: vec![],
        nullifiers: vec![NullifierUpdate {
            nullifier: Some(recorded_notes.first().unwrap().note().nullifier().into()),
            block_num: 7,
        }],
    };
    requests.insert(request, response);

    requests
}

/// inserts mock note and account data into the client
pub fn insert_mock_data(client: &mut Client<impl DataStore>) {
    use mock::mock::{
        account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs,
    };

    // generate test data
    let (account, _, _, recorded_notes) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    // insert notes into database
    for note in recorded_notes.into_iter() {
        client.insert_input_note(note).unwrap();
    }

    let _transaction = mock_executed_tx(AssetPreservationStatus::Preserved);

    // insert account
    let auth_info = AuthInfo::RpoFalcon512(
        KeyPair::new()
            .map_err(|err| format!("Error generating KeyPair: {}", err))
            .unwrap(),
    );

    //client
    //    .insert_account(transaction.initial_account(), &auth_info)
    //    .unwrap();
    //client.insert_transaction(transaction).unwrap();

    client.insert_account(&account, &auth_info).unwrap();
}
