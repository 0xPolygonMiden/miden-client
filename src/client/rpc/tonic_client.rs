use async_trait::async_trait;
use miden_node_proto::{
    errors::ConversionError,
    generated::{
        requests::{
            GetAccountDetailsRequest, GetBlockHeaderByNumberRequest,
            SubmitProvenTransactionRequest, SyncStateRequest,
        },
        responses::SyncStateResponse,
        rpc::api_client::ApiClient,
    },
};
use miden_objects::{
    accounts::{Account, AccountId},
    notes::{NoteId, NoteMetadata, NoteType},
    transaction::ProvenTransaction,
    utils::Deserializable,
    BlockHeader, Digest,
};
use miden_tx::utils::Serializable;
use tonic::transport::Channel;

use super::{CommittedNote, NodeRpcClient, NodeRpcClientEndpoint, StateSyncInfo};
use crate::errors::NodeRpcClientError;

// TONIC RPC CLIENT
// ================================================================================================

/// Client for the Node RPC API using tonic
///
/// Wraps the ApiClient which defers establishing a connection with a node until necessary
pub struct TonicRpcClient {
    rpc_api: Option<ApiClient<Channel>>,
    endpoint: String,
}

impl TonicRpcClient {
    /// Returns a new instance of [TonicRpcClient] that'll do calls the `config_endpoint` provided
    pub fn new(config_endpoint: &str) -> TonicRpcClient {
        TonicRpcClient {
            rpc_api: None,
            endpoint: config_endpoint.to_string(),
        }
    }

    /// Takes care of establishing the RPC connection if not connected yet and returns a reference
    /// to the inner ApiClient
    async fn rpc_api(&mut self) -> Result<&mut ApiClient<Channel>, NodeRpcClientError> {
        if self.rpc_api.is_some() {
            Ok(self.rpc_api.as_mut().unwrap())
        } else {
            let rpc_api = ApiClient::connect(self.endpoint.clone())
                .await
                .map_err(|err| NodeRpcClientError::ConnectionError(err.to_string()))?;
            Ok(self.rpc_api.insert(rpc_api))
        }
    }
}

#[async_trait]
impl NodeRpcClient for TonicRpcClient {
    async fn submit_proven_transaction(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), NodeRpcClientError> {
        let request = SubmitProvenTransactionRequest {
            transaction: proven_transaction.to_bytes(),
        };
        let rpc_api = self.rpc_api().await?;
        rpc_api.submit_proven_transaction(request).await.map_err(|err| {
            NodeRpcClientError::RequestError(
                NodeRpcClientEndpoint::SubmitProvenTx.to_string(),
                err.to_string(),
            )
        })?;

        Ok(())
    }

    async fn get_block_header_by_number(
        &mut self,
        block_num: Option<u32>,
    ) -> Result<BlockHeader, NodeRpcClientError> {
        let request = GetBlockHeaderByNumberRequest { block_num };
        let rpc_api = self.rpc_api().await?;
        let api_response = rpc_api.get_block_header_by_number(request).await.map_err(|err| {
            NodeRpcClientError::RequestError(
                NodeRpcClientEndpoint::GetBlockHeaderByNumber.to_string(),
                err.to_string(),
            )
        })?;

        api_response
            .into_inner()
            .block_header
            .ok_or(NodeRpcClientError::ExpectedFieldMissing("BlockHeader".into()))?
            .try_into()
            .map_err(|err: ConversionError| NodeRpcClientError::ConversionFailure(err.to_string()))
    }

    /// Sends a sync state request to the Miden node, validates and converts the response
    /// into a [StateSyncInfo] struct.
    async fn sync_state(
        &mut self,
        block_num: u32,
        account_ids: &[AccountId],
        note_tags: &[u16],
        nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, NodeRpcClientError> {
        let account_ids = account_ids.iter().map(|acc| (*acc).into()).collect();

        let nullifiers = nullifiers_tags.iter().map(|&nullifier| nullifier as u32).collect();

        let note_tags = note_tags.iter().map(|&note_tag| note_tag as u32).collect();

        let request = SyncStateRequest {
            block_num,
            account_ids,
            note_tags,
            nullifiers,
        };

        let rpc_api = self.rpc_api().await?;
        let response = rpc_api.sync_state(request).await.map_err(|err| {
            NodeRpcClientError::RequestError(
                NodeRpcClientEndpoint::SyncState.to_string(),
                err.to_string(),
            )
        })?;
        response.into_inner().try_into()
    }

    /// TODO: fill description
    async fn get_account_update(
        &mut self,
        account_id: AccountId,
    ) -> Result<Option<Account>, NodeRpcClientError> {
        debug_assert!(account_id.is_on_chain());

        let account_id = account_id.into();
        let request = GetAccountDetailsRequest {
            account_id: Some(account_id),
        };

        let rpc_api = self.rpc_api().await?;

        let response = rpc_api.get_account_details(request).await.map_err(|err| {
            NodeRpcClientError::RequestError(
                NodeRpcClientEndpoint::GetAccountDetails.to_string(),
                err.to_string(),
            )
        })?;
        let response = dbg!(response.into_inner());
        // TODO: remove unwrap and use proper handling
        let account_info = response.account.unwrap();
        let details = account_info
            .details
            .map(|details| Account::read_from_bytes(&details))
            .transpose()?;

        Ok(details)
    }
}

// STATE SYNC INFO CONVERSION
// ================================================================================================

impl TryFrom<SyncStateResponse> for StateSyncInfo {
    type Error = NodeRpcClientError;

    fn try_from(value: SyncStateResponse) -> Result<Self, Self::Error> {
        let chain_tip = value.chain_tip;

        // Validate and convert block header
        let block_header = value
            .block_header
            .ok_or(NodeRpcClientError::ExpectedFieldMissing("BlockHeader".into()))?
            .try_into()?;

        // Validate and convert MMR Delta
        let mmr_delta = value
            .mmr_delta
            .ok_or(NodeRpcClientError::ExpectedFieldMissing("MmrDelta".into()))?
            .try_into()?;

        // Validate and convert account hash updates into an (AccountId, Digest) tuple
        let mut account_hash_updates = vec![];
        for update in value.accounts {
            let account_id = update
                .account_id
                .ok_or(NodeRpcClientError::ExpectedFieldMissing(
                    "AccountHashUpdate.AccountId".into(),
                ))?
                .try_into()?;
            let account_hash = update
                .account_hash
                .ok_or(NodeRpcClientError::ExpectedFieldMissing(
                    "AccountHashUpdate.AccountHash".into(),
                ))?
                .try_into()?;
            account_hash_updates.push((account_id, account_hash));
        }

        // Validate and convert account note inclusions into an (AccountId, Digest) tuple
        let mut note_inclusions = vec![];
        for note in value.notes {
            let note_id: Digest = note
                .note_id
                .ok_or(NodeRpcClientError::ExpectedFieldMissing("Notes.Id".into()))?
                .try_into()?;

            let note_id: NoteId = note_id.into();

            let merkle_path = note
                .merkle_path
                .ok_or(NodeRpcClientError::ExpectedFieldMissing("Notes.MerklePath".into()))?
                .try_into()?;

            let sender_account_id = note
                .sender
                .ok_or(NodeRpcClientError::ExpectedFieldMissing("Notes.Sender".into()))?
                .try_into()?;
            // TODO: Note type and aux should eventually come from the node response
            // TODO: Remove unwraps
            let metadata = NoteMetadata::new(
                sender_account_id,
                NoteType::OffChain,
                note.tag.try_into().unwrap(),
                Default::default(),
            )
            .unwrap();

            let committed_note =
                CommittedNote::new(note_id, note.note_index, merkle_path, metadata);

            note_inclusions.push(committed_note);
        }

        let nullifiers = value
            .nullifiers
            .iter()
            .map(|nul_update| {
                nul_update
                    .clone()
                    .nullifier
                    .ok_or(NodeRpcClientError::ExpectedFieldMissing("Nullifier".into()))
                    .and_then(|n| {
                        Digest::try_from(n)
                            .map_err(|err| NodeRpcClientError::ConversionFailure(err.to_string()))
                    })
            })
            .collect::<Result<Vec<Digest>, NodeRpcClientError>>()?;

        Ok(Self {
            chain_tip,
            block_header,
            mmr_delta,
            account_hash_updates,
            note_inclusions,
            nullifiers,
        })
    }
}
