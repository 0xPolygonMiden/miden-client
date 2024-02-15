use core::fmt;
use crypto::{
    merkle::{MerklePath, MmrDelta},
    utils::DeserializationError,
};
use miden_node_proto::generated::responses::SyncStateResponse;
use objects::{
    accounts::AccountId,
    notes::{NoteId, NoteMetadata},
    BlockHeader, Digest,
};

// STATE SYNC INFO
// ================================================================================================

/// Represents a [SyncStateResponse] with fields converted into domain types
pub struct StateSyncInfo {
    /// The block number of the chain tip at the moment of the response
    pub chain_tip: u32,
    /// The returned block header
    pub block_header: BlockHeader,
    /// MMR delta that contains data for (current_block.num, incoming_block_header.num-1)
    pub mmr_delta: MmrDelta,
    /// Tuples of AccountId alongside their new account hashes
    pub account_hash_updates: Vec<(AccountId, Digest)>,
    /// List of tuples of Note ID, Note Index and Merkle Path for all new notes
    pub note_inclusions: Vec<CommittedNote>,
    /// List of nullifiers that identify spent notes
    pub nullifiers: Vec<Digest>,
}

impl TryFrom<SyncStateResponse> for StateSyncInfo {
    type Error = RpcApiError;

    fn try_from(value: SyncStateResponse) -> Result<Self, Self::Error> {
        let chain_tip = value.chain_tip;

        // Validate and convert block header
        let block_header = value
            .block_header
            .ok_or(RpcApiError::ExpectedFieldMissing("BlockHeader".into()))?
            .try_into()?;

        // Validate and convert MMR Delta
        let mmr_delta = value
            .mmr_delta
            .ok_or(RpcApiError::ExpectedFieldMissing("MmrDelta".into()))?
            .try_into()?;

        // Validate and convert account hash updates into an (AccountId, Digest) tuple
        let mut account_hash_updates = vec![];
        for update in value.accounts {
            let account_id = update
                .account_id
                .ok_or(RpcApiError::ExpectedFieldMissing(
                    "AccountHashUpdate.AccountId".into(),
                ))?
                .try_into()?;
            let account_hash = update
                .account_hash
                .ok_or(RpcApiError::ExpectedFieldMissing(
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
                .ok_or(RpcApiError::ExpectedFieldMissing("Notes.Id".into()))?
                .try_into()?;
            let note_id: NoteId = note_id.into();

            let merkle_path = note
                .merkle_path
                .ok_or(RpcApiError::ExpectedFieldMissing("Notes.MerklePath".into()))?
                .try_into()?;

            let sender_account_id = note
                .sender
                .ok_or(RpcApiError::ExpectedFieldMissing("Notes.Sender".into()))?
                .try_into()?;
            let metadata = NoteMetadata::new(
                sender_account_id,
                note.tag
                    .try_into()
                    .map_err(DeserializationError::InvalidValue)?,
            );

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
                    .ok_or(RpcApiError::ExpectedFieldMissing("Nullifier".into()))
                    .and_then(|n| Digest::try_from(n).map_err(RpcApiError::ConversionFailure))
            })
            .collect::<Result<Vec<Digest>, RpcApiError>>()?;

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

// COMMITTED NOTE
// ================================================================================================

/// Represents a committed note, returned as part of a [SyncStateResponse]
pub struct CommittedNote {
    /// Note ID of the committed note
    note_id: NoteId,
    /// Note index for the note merkle tree
    note_index: u32,
    /// Merkle path for the note merkle tree up to the block's note root
    merkle_path: MerklePath,
    /// Note metadata
    metadata: NoteMetadata,
}

impl CommittedNote {
    pub fn new(
        note_id: NoteId,
        note_index: u32,
        merkle_path: MerklePath,
        metadata: NoteMetadata,
    ) -> Self {
        Self {
            note_id,
            note_index,
            merkle_path,
            metadata,
        }
    }

    pub fn note_id(&self) -> &NoteId {
        &self.note_id
    }

    pub fn note_index(&self) -> u32 {
        self.note_index
    }

    pub fn merkle_path(&self) -> &MerklePath {
        &self.merkle_path
    }

    pub fn metadata(&self) -> NoteMetadata {
        self.metadata
    }
}

// RPC CLIENT
// ================================================================================================
//
#[cfg(not(any(test, feature = "mock")))]
pub(crate) use client::RpcClient;

use crate::errors::RpcApiError;

#[cfg(not(any(test, feature = "mock")))]
mod client {
    use super::{RpcApiEndpoint, StateSyncInfo};
    use crate::errors::RpcApiError;
    use miden_node_proto::generated::{
        requests::{
            GetBlockHeaderByNumberRequest, SubmitProvenTransactionRequest, SyncStateRequest,
        },
        responses::SubmitProvenTransactionResponse,
        rpc::api_client::ApiClient,
    };
    use objects::{accounts::AccountId, BlockHeader};
    use tonic::transport::Channel;

    /// Wrapper for ApiClient which defers establishing a connection with a node until necessary
    pub(crate) struct RpcClient {
        rpc_api: Option<ApiClient<Channel>>,
        endpoint: String,
    }

    impl RpcClient {
        pub fn new(config_endpoint: String) -> RpcClient {
            RpcClient {
                rpc_api: None,
                endpoint: config_endpoint,
            }
        }

        pub async fn submit_proven_transaction(
            &mut self,
            request: impl tonic::IntoRequest<SubmitProvenTransactionRequest>,
        ) -> Result<tonic::Response<SubmitProvenTransactionResponse>, RpcApiError> {
            let rpc_api = self.rpc_api().await?;
            rpc_api
                .submit_proven_transaction(request)
                .await
                .map_err(|err| RpcApiError::RequestError(RpcApiEndpoint::SubmitProvenTx, err))
        }

        pub async fn get_block_header_by_number(
            &mut self,
            request: impl tonic::IntoRequest<GetBlockHeaderByNumberRequest>,
        ) -> Result<BlockHeader, RpcApiError> {
            let rpc_api = self.rpc_api().await?;
            let api_response =
                rpc_api
                    .get_block_header_by_number(request)
                    .await
                    .map_err(|err| {
                        RpcApiError::RequestError(RpcApiEndpoint::GetBlockHeaderByNumber, err)
                    })?;

            api_response
                .into_inner()
                .block_header
                .ok_or(RpcApiError::ExpectedFieldMissing("BlockHeader".into()))?
                .try_into()
                .map_err(RpcApiError::ConversionFailure)
        }

        /// Takes care of establishing the RPC connection if not connected yet and returns a reference
        /// to the inner ApiClient
        async fn rpc_api(&mut self) -> Result<&mut ApiClient<Channel>, RpcApiError> {
            if self.rpc_api.is_some() {
                Ok(self.rpc_api.as_mut().unwrap())
            } else {
                let rpc_api = ApiClient::connect(self.endpoint.clone())
                    .await
                    .map_err(RpcApiError::ConnectionError)?;
                Ok(self.rpc_api.insert(rpc_api))
            }
        }

        /// Sends a sync state request to the Miden node, validates and converts the response
        /// into a [StateSyncInfo] struct.
        pub async fn sync_state(
            &mut self,
            block_num: u32,
            account_ids: &Vec<AccountId>,
            note_tags: &[u16],
            nullifiers_tags: &[u16],
        ) -> Result<StateSyncInfo, RpcApiError> {
            let account_ids = account_ids.iter().map(|acc| (*acc).into()).collect();

            let nullifiers = nullifiers_tags
                .iter()
                .map(|&nullifier| nullifier as u32)
                .collect();

            let note_tags = note_tags.iter().map(|&note_tag| note_tag as u32).collect();

            let request = SyncStateRequest {
                block_num,
                account_ids,
                note_tags,
                nullifiers,
            };

            let rpc_api = self.rpc_api().await?;
            let response = rpc_api
                .sync_state(request)
                .await
                .map_err(|err| RpcApiError::RequestError(RpcApiEndpoint::SyncState, err))?;
            response.into_inner().try_into()
        }
    }
}

// RPC API ENDPOINT
// ================================================================================================
//
#[derive(Debug)]
pub enum RpcApiEndpoint {
    GetBlockHeaderByNumber,
    SyncState,
    SubmitProvenTx,
}

impl fmt::Display for RpcApiEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcApiEndpoint::GetBlockHeaderByNumber => write!(f, "get_block_header_by_number"),
            RpcApiEndpoint::SyncState => write!(f, "sync_state"),
            RpcApiEndpoint::SubmitProvenTx => write!(f, "submit_proven_transaction"),
        }
    }
}
