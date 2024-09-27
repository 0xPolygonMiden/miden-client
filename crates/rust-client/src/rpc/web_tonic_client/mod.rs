use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

use async_trait::async_trait;
use generated::{
    requests::{
        CheckNullifiersByPrefixRequest, GetAccountDetailsRequest, GetBlockHeaderByNumberRequest,
        GetNotesByIdRequest, SubmitProvenTransactionRequest, SyncNoteRequest, SyncStateRequest,
    },
    responses::{SyncNoteResponse, SyncStateResponse},
    rpc::api_client::ApiClient,
};
use miden_objects::{
    accounts::{Account, AccountId},
    crypto::merkle::{MerklePath, MmrProof},
    notes::{Note, NoteId, NoteTag, Nullifier},
    transaction::{ProvenTransaction, TransactionId},
    utils::Deserializable,
    BlockHeader, Digest,
};
use miden_tx::utils::Serializable;
use tonic_web_wasm_client::Client;

use super::NoteSyncInfo;
use crate::rpc::{
    AccountDetails, AccountUpdateSummary, CommittedNote, NodeRpcClient, NodeRpcClientEndpoint,
    NoteDetails, NoteInclusionDetails, NullifierUpdate, RpcError, StateSyncInfo, TransactionUpdate,
};

#[rustfmt::skip]
pub mod generated;

pub struct WebTonicRpcClient {
    endpoint: String,
}

impl WebTonicRpcClient {
    pub fn new(endpoint: &str) -> Self {
        Self { endpoint: endpoint.to_string() }
    }

    pub fn build_api_client(&self) -> ApiClient<Client> {
        let wasm_client = Client::new(self.endpoint.clone());
        ApiClient::new(wasm_client)
    }
}

#[async_trait]
impl NodeRpcClient for WebTonicRpcClient {
    async fn submit_proven_transaction(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), RpcError> {
        let mut query_client = self.build_api_client();

        let request = SubmitProvenTransactionRequest {
            transaction: proven_transaction.to_bytes(),
        };

        query_client.submit_proven_transaction(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::SubmitProvenTx.to_string(),
                err.to_string(),
            )
        })?;

        Ok(())
    }

    async fn get_block_header_by_number(
        &mut self,
        block_num: Option<u32>,
        include_mmr_proof: bool,
    ) -> Result<(BlockHeader, Option<MmrProof>), RpcError> {
        let mut query_client = self.build_api_client();

        let request = GetBlockHeaderByNumberRequest {
            block_num,
            include_mmr_proof: Some(include_mmr_proof),
        };

        // Attempt to send the request and process the response
        let api_response =
            query_client.get_block_header_by_number(request).await.map_err(|err| {
                // log to console all the properties of block header
                RpcError::RequestError(
                    NodeRpcClientEndpoint::GetBlockHeaderByNumber.to_string(),
                    err.to_string(),
                )
            })?;

        let response = api_response.into_inner();

        let block_header: BlockHeader = response
            .block_header
            .ok_or(RpcError::ExpectedFieldMissing("BlockHeader".into()))?
            .try_into()?;

        let mmr_proof = if include_mmr_proof {
            let forest = response
                .chain_length
                .ok_or(RpcError::ExpectedFieldMissing("ChainLength".into()))?;
            let merkle_path: MerklePath = response
                .mmr_path
                .ok_or(RpcError::ExpectedFieldMissing("MmrPath".into()))?
                .try_into()?;

            Some(MmrProof {
                forest: forest as usize,
                position: block_header.block_num() as usize,
                merkle_path,
            })
        } else {
            None
        };

        Ok((block_header, mmr_proof))
    }

    async fn get_notes_by_id(&mut self, note_ids: &[NoteId]) -> Result<Vec<NoteDetails>, RpcError> {
        let mut query_client = self.build_api_client();

        let request = GetNotesByIdRequest {
            note_ids: note_ids.iter().map(|id| id.inner().into()).collect(),
        };

        let api_response = query_client.get_notes_by_id(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::GetBlockHeaderByNumber.to_string(),
                err.to_string(),
            )
        })?;

        let rpc_notes = api_response.into_inner().notes;
        let mut response_notes = Vec::with_capacity(rpc_notes.len());
        for note in rpc_notes {
            let inclusion_details = {
                let merkle_path = note
                    .merkle_path
                    .ok_or(RpcError::ExpectedFieldMissing("Notes.MerklePath".into()))?
                    .try_into()?;

                NoteInclusionDetails::new(note.block_num, note.note_index as u16, merkle_path)
            };

            let note = match note.details {
                // On-chain notes include details
                Some(details) => {
                    let note = Note::read_from_bytes(&details)?;

                    NoteDetails::Public(note, inclusion_details)
                },
                // Off-chain notes do not have details
                None => {
                    let note_metadata = note
                        .metadata
                        .ok_or(RpcError::ExpectedFieldMissing("Metadata".into()))?
                        .try_into()?;
                    let note_id: miden_objects::Digest = note
                        .note_id
                        .ok_or(RpcError::ExpectedFieldMissing("Notes.NoteId".into()))?
                        .try_into()?;

                    NoteDetails::Private(NoteId::from(note_id), note_metadata, inclusion_details)
                },
            };
            response_notes.push(note)
        }
        Ok(response_notes)
    }

    /// Sends a sync state request to the Miden node, validates and converts the response
    /// into a [StateSyncInfo] struct.
    async fn sync_state(
        &mut self,
        block_num: u32,
        account_ids: &[AccountId],
        note_tags: &[NoteTag],
        nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, RpcError> {
        let mut query_client = self.build_api_client();

        let account_ids = account_ids.iter().map(|acc| (*acc).into()).collect();
        let nullifiers = nullifiers_tags.iter().map(|&nullifier| nullifier as u32).collect();
        let note_tags = note_tags.iter().map(|&note_tag| note_tag.into()).collect();

        let request = SyncStateRequest {
            block_num,
            account_ids,
            note_tags,
            nullifiers,
        };

        let response = query_client.sync_state(request).await.map_err(|err| {
            RpcError::RequestError(NodeRpcClientEndpoint::SyncState.to_string(), err.to_string())
        })?;
        response.into_inner().try_into()
    }

    async fn sync_notes(
        &mut self,
        block_num: u32,
        note_tags: &[NoteTag],
    ) -> Result<NoteSyncInfo, RpcError> {
        let mut query_client = self.build_api_client();

        let note_tags = note_tags.iter().map(|&note_tag| note_tag.into()).collect();

        let request = SyncNoteRequest { block_num, note_tags };

        let response = query_client.sync_notes(request).await.map_err(|err| {
            RpcError::RequestError(NodeRpcClientEndpoint::SyncState.to_string(), err.to_string())
        })?;
        response.into_inner().try_into()
    }

    /// Sends a [GetAccountDetailsRequest] to the Miden node, and extracts an [Account] from the
    /// `GetAccountDetailsResponse` response.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// - The provided account is not on-chain: this is due to the fact that for offchain accounts
    ///   the client is responsible
    /// - There was an error sending the request to the node
    /// - The answer had a `None` for its account, or the account had a `None` at the `details`
    ///   field.
    /// - There is an error during [Account] deserialization
    async fn get_account_update(
        &mut self,
        account_id: AccountId,
    ) -> Result<AccountDetails, RpcError> {
        let mut query_client = self.build_api_client();

        let request = GetAccountDetailsRequest { account_id: Some(account_id.into()) };

        let response = query_client.get_account_details(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::GetAccountDetails.to_string(),
                err.to_string(),
            )
        })?;

        let response = response.into_inner();
        let account_info = response.details.ok_or(RpcError::ExpectedFieldMissing(
            "GetAccountDetails response should have an `account`".to_string(),
        ))?;

        let account_summary = account_info.summary.ok_or(RpcError::ExpectedFieldMissing(
            "GetAccountDetails response's account should have a `summary`".to_string(),
        ))?;

        let hash = account_summary.account_hash.ok_or(RpcError::ExpectedFieldMissing(
            "GetAccountDetails response's account should have an `account_hash`".to_string(),
        ))?;

        let hash = hash.try_into()?;

        let update_summary = AccountUpdateSummary::new(hash, account_summary.block_num);
        if account_id.is_public() {
            let details_bytes = account_info.details.ok_or(RpcError::ExpectedFieldMissing(
                "GetAccountDetails response's account should have `details`".to_string(),
            ))?;

            let account = Account::read_from_bytes(&details_bytes)?;

            Ok(AccountDetails::Public(account, update_summary))
        } else {
            Ok(AccountDetails::Private(account_id, update_summary))
        }
    }

    async fn check_nullifiers_by_prefix(
        &mut self,
        prefixes: &[u16],
    ) -> Result<Vec<(Nullifier, u32)>, RpcError> {
        let mut query_client = self.build_api_client();

        let request = CheckNullifiersByPrefixRequest {
            nullifiers: prefixes.iter().map(|&x| x as u32).collect(),
            prefix_len: 16,
        };

        let response = query_client.check_nullifiers_by_prefix(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::CheckNullifiersByPrefix.to_string(),
                err.to_string(),
            )
        })?;

        let response = response.into_inner();
        let nullifiers = response
            .nullifiers
            .iter()
            .map(|nul| {
                let nullifier = nul.nullifier.ok_or(RpcError::ExpectedFieldMissing(
                    "CheckNullifiersByPrefix response should have a `nullifier`".to_string(),
                ))?;
                let nullifier = nullifier.try_into()?;
                Ok((nullifier, nul.block_num))
            })
            .collect::<Result<Vec<(Nullifier, u32)>, RpcError>>()?;
        Ok(nullifiers)
    }
}

// NOTE SYNC INFO CONVERSION
// ================================================================================================

impl TryFrom<SyncNoteResponse> for NoteSyncInfo {
    type Error = RpcError;

    fn try_from(value: SyncNoteResponse) -> Result<Self, Self::Error> {
        let chain_tip = value.chain_tip;

        // Validate and convert block header
        // Validate and convert block header
        let block_header = value
            .block_header
            .ok_or(RpcError::ExpectedFieldMissing("BlockHeader".into()))?
            .try_into()?;

        let mmr_path = value
            .mmr_path
            .ok_or(RpcError::ExpectedFieldMissing("MmrPath".into()))?
            .try_into()?;

        // Validate and convert account note inclusions into an (AccountId, Digest) tuple
        let mut notes = vec![];
        for note in value.notes {
            let note_id: Digest = note
                .note_id
                .ok_or(RpcError::ExpectedFieldMissing("Notes.Id".into()))?
                .try_into()?;

            let note_id: NoteId = note_id.into();

            let merkle_path = note
                .merkle_path
                .ok_or(RpcError::ExpectedFieldMissing("Notes.MerklePath".into()))?
                .try_into()?;

            let metadata = note
                .metadata
                .ok_or(RpcError::ExpectedFieldMissing("Metadata".into()))?
                .try_into()?;

            let committed_note =
                CommittedNote::new(note_id, note.note_index as u16, merkle_path, metadata);

            notes.push(committed_note);
        }

        Ok(NoteSyncInfo { chain_tip, block_header, mmr_path, notes })
    }
}

// STATE SYNC INFO CONVERSION
// ================================================================================================

impl TryFrom<SyncStateResponse> for StateSyncInfo {
    type Error = RpcError;

    fn try_from(value: SyncStateResponse) -> Result<Self, Self::Error> {
        let chain_tip = value.chain_tip;

        // Validate and convert block header
        let block_header: BlockHeader = value
            .block_header
            .ok_or(RpcError::ExpectedFieldMissing("BlockHeader".into()))?
            .try_into()?;

        // Validate and convert MMR Delta
        let mmr_delta = value
            .mmr_delta
            .ok_or(RpcError::ExpectedFieldMissing("MmrDelta".into()))?
            .try_into()?;

        // Validate and convert account hash updates into an (AccountId, Digest) tuple
        let mut account_hash_updates = vec![];
        for update in value.accounts {
            let account_id = update
                .account_id
                .ok_or(RpcError::ExpectedFieldMissing("AccountHashUpdate.AccountId".into()))?
                .try_into()?;
            let account_hash = update
                .account_hash
                .ok_or(RpcError::ExpectedFieldMissing("AccountHashUpdate.AccountHash".into()))?
                .try_into()?;
            account_hash_updates.push((account_id, account_hash));
        }

        // Validate and convert account note inclusions into an (AccountId, Digest) tuple
        let mut note_inclusions = vec![];
        for note in value.notes {
            let note_id: Digest = note
                .note_id
                .ok_or(RpcError::ExpectedFieldMissing("Notes.Id".into()))?
                .try_into()?;

            let note_id: NoteId = note_id.into();

            let merkle_path = note
                .merkle_path
                .ok_or(RpcError::ExpectedFieldMissing("Notes.MerklePath".into()))?
                .try_into()?;

            let metadata = note
                .metadata
                .ok_or(RpcError::ExpectedFieldMissing("Metadata".into()))?
                .try_into()?;

            let committed_note =
                CommittedNote::new(note_id, note.note_index as u16, merkle_path, metadata);

            note_inclusions.push(committed_note);
        }

        let nullifiers = value
            .nullifiers
            .iter()
            .map(|nul_update| {
                let nullifier_digest = nul_update
                    .nullifier
                    .ok_or(RpcError::ExpectedFieldMissing("Nullifier".into()))?;

                let nullifier_digest = Digest::try_from(nullifier_digest)?;

                let nullifier_block_num = nul_update.block_num;

                Ok(NullifierUpdate {
                    nullifier: nullifier_digest.into(),
                    block_num: nullifier_block_num,
                })
            })
            .collect::<Result<Vec<NullifierUpdate>, RpcError>>()?;

        let transactions = value
            .transactions
            .iter()
            .map(|transaction_summary| {
                let transaction_id = transaction_summary.transaction_id.ok_or(
                    RpcError::ExpectedFieldMissing("TransactionSummary.TransactionId".into()),
                )?;
                let transaction_id = TransactionId::try_from(transaction_id)?;

                let transaction_block_num = transaction_summary.block_num;

                let transaction_account_id = transaction_summary.account_id.ok_or(
                    RpcError::ExpectedFieldMissing("TransactionSummary.TransactionId".into()),
                )?;
                let transaction_account_id = AccountId::try_from(transaction_account_id)?;

                Ok(TransactionUpdate {
                    transaction_id,
                    block_num: transaction_block_num,
                    account_id: transaction_account_id,
                })
            })
            .collect::<Result<Vec<TransactionUpdate>, RpcError>>()?;

        Ok(Self {
            chain_tip,
            block_header,
            mmr_delta,
            account_hash_updates,
            note_inclusions,
            nullifiers,
            transactions,
        })
    }
}
