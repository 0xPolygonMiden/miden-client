use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};
#[allow(unused_imports)]
use core::time::Duration;

use async_trait::async_trait;
use miden_objects::{
    account::{Account, AccountCode, AccountDelta, AccountId},
    block::{BlockHeader, BlockNumber, ProvenBlock},
    crypto::merkle::{MerklePath, MmrProof, SmtProof},
    note::{Note, NoteId, NoteInclusionProof, NoteTag, Nullifier},
    transaction::ProvenTransaction,
    utils::Deserializable,
    Digest,
};
use miden_tx::utils::Serializable;
use tracing::info;

use super::{
    domain::{
        account::{AccountProof, AccountProofs, AccountUpdateSummary},
        note::NetworkNote,
        nullifier::NullifierUpdate,
    },
    generated::{
        requests::{
            get_account_proofs_request::{self},
            CheckNullifiersByPrefixRequest, CheckNullifiersRequest, GetAccountDetailsRequest,
            GetAccountProofsRequest, GetAccountStateDeltaRequest, GetBlockByNumberRequest,
            GetNotesByIdRequest, SubmitProvenTransactionRequest, SyncNoteRequest, SyncStateRequest,
        },
        rpc::api_client::ApiClient,
    },
    AccountDetails, Endpoint, NodeRpcClient, NodeRpcClientEndpoint, NoteSyncInfo, RpcError,
    StateSyncInfo,
};
use crate::{rpc::generated::requests::GetBlockHeaderByNumberRequest, transaction::ForeignAccount};

// TONIC RPC CLIENT
// ================================================================================================

#[cfg(feature = "web-tonic")]
type InnerClient = tonic_web_wasm_client::Client;
#[cfg(feature = "tonic")]
type InnerClient = tonic::transport::Channel;

/// Client for the Node RPC API using tonic.
///
/// If the `tonic` feature is enabled, this client will use a `tonic::transport::Channel` to
/// communicate with the node. In this case the connection will be established lazily when the
/// first request is made.
/// If the `web-tonic` feature is enabled, this client will use a `tonic_web_wasm_client::Client` to
/// communicate with the node.
///
/// In both cases, the `TonicRpcClient` depends on the types inside the `generated` module, which
/// are generated by the build script and also depend on the target architecture.
pub struct TonicRpcClient {
    client: Option<ApiClient<InnerClient>>,
    endpoint: String,
    #[allow(dead_code)]
    timeout_ms: u64,
}

impl TonicRpcClient {
    /// Returns a new instance of [`TonicRpcClient`] that'll do calls to the provided [`Endpoint`]
    /// with the given timeout in milliseconds.
    pub fn new(endpoint: &Endpoint, timeout_ms: u64) -> TonicRpcClient {
        TonicRpcClient {
            client: None,
            endpoint: endpoint.to_string(),
            timeout_ms,
        }
    }

    /// Takes care of establishing the RPC connection if not connected yet and returns a reference
    /// to the inner `ApiClient`.
    #[allow(clippy::unused_async)]
    async fn rpc_api(&mut self) -> Result<&mut ApiClient<InnerClient>, RpcError> {
        if self.client.is_some() {
            Ok(self.client.as_mut().unwrap())
        } else {
            #[cfg(target_arch = "wasm32")]
            let new_client = {
                let wasm_client = tonic_web_wasm_client::Client::new(self.endpoint.clone());
                ApiClient::new(wasm_client)
            };

            #[cfg(not(target_arch = "wasm32"))]
            let new_client = {
                let endpoint = tonic::transport::Endpoint::try_from(self.endpoint.clone())
                    .map_err(|err| RpcError::ConnectionError(err.to_string()))?
                    .timeout(Duration::from_millis(self.timeout_ms));

                ApiClient::connect(endpoint)
                    .await
                    .map_err(|err| RpcError::ConnectionError(err.to_string()))?
            };

            Ok(self.client.insert(new_client))
        }
    }
}

#[async_trait(?Send)]
impl NodeRpcClient for TonicRpcClient {
    async fn submit_proven_transaction(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), RpcError> {
        let request = SubmitProvenTransactionRequest {
            transaction: proven_transaction.to_bytes(),
        };
        let rpc_api = self.rpc_api().await?;
        rpc_api.submit_proven_transaction(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::SubmitProvenTx.to_string(),
                err.to_string(),
            )
        })?;

        Ok(())
    }

    async fn get_block_header_by_number(
        &mut self,
        block_num: Option<BlockNumber>,
        include_mmr_proof: bool,
    ) -> Result<(BlockHeader, Option<MmrProof>), RpcError> {
        let request = GetBlockHeaderByNumberRequest {
            block_num: block_num.as_ref().map(BlockNumber::as_u32),
            include_mmr_proof: Some(include_mmr_proof),
        };

        info!("Calling GetBlockHeaderByNumber: {:?}", request);

        let rpc_api = self.rpc_api().await?;
        let api_response = rpc_api.get_block_header_by_number(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::GetBlockHeaderByNumber.to_string(),
                err.to_string(),
            )
        })?;

        let response = api_response.into_inner();

        let block_header: BlockHeader = response
            .block_header
            .ok_or(RpcError::ExpectedDataMissing("BlockHeader".into()))?
            .try_into()?;

        let mmr_proof = if include_mmr_proof {
            let forest = response
                .chain_length
                .ok_or(RpcError::ExpectedDataMissing("ChainLength".into()))?;
            let merkle_path: MerklePath = response
                .mmr_path
                .ok_or(RpcError::ExpectedDataMissing("MmrPath".into()))?
                .try_into()?;

            Some(MmrProof {
                forest: forest as usize,
                position: block_header.block_num().as_usize(),
                merkle_path,
            })
        } else {
            None
        };

        Ok((block_header, mmr_proof))
    }

    async fn get_notes_by_id(&mut self, note_ids: &[NoteId]) -> Result<Vec<NetworkNote>, RpcError> {
        let request = GetNotesByIdRequest {
            note_ids: note_ids.iter().map(|id| id.inner().into()).collect(),
        };
        let rpc_api = self.rpc_api().await?;
        let api_response = rpc_api.get_notes_by_id(request).await.map_err(|err| {
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
                    .ok_or(RpcError::ExpectedDataMissing("Notes.MerklePath".into()))?
                    .try_into()?;

                NoteInclusionProof::new(
                    note.block_num.into(),
                    u16::try_from(note.note_index).expect("note index out of range"),
                    merkle_path,
                )?
            };

            let note = if let Some(details) = note.details {
                let note = Note::read_from_bytes(&details)?;

                NetworkNote::Public(note, inclusion_details)
            } else {
                let note_metadata = note
                    .metadata
                    .ok_or(RpcError::ExpectedDataMissing("Metadata".into()))?
                    .try_into()?;

                let note_id: Digest = note
                    .note_id
                    .ok_or(RpcError::ExpectedDataMissing("Notes.NoteId".into()))?
                    .try_into()?;

                NetworkNote::Private(NoteId::from(note_id), note_metadata, inclusion_details)
            };
            response_notes.push(note);
        }
        Ok(response_notes)
    }

    /// Sends a sync state request to the Miden node, validates and converts the response
    /// into a [StateSyncInfo] struct.
    async fn sync_state(
        &mut self,
        block_num: BlockNumber,
        account_ids: &[AccountId],
        note_tags: &[NoteTag],
    ) -> Result<StateSyncInfo, RpcError> {
        let account_ids = account_ids.iter().map(|acc| (*acc).into()).collect();

        let note_tags = note_tags.iter().map(|&note_tag| note_tag.into()).collect();

        let request = SyncStateRequest {
            block_num: block_num.as_u32(),
            account_ids,
            note_tags,
        };

        let rpc_api = self.rpc_api().await?;
        let response = rpc_api.sync_state(request).await.map_err(|err| {
            RpcError::RequestError(NodeRpcClientEndpoint::SyncState.to_string(), err.to_string())
        })?;
        response.into_inner().try_into()
    }

    /// Sends a `GetAccountDetailsRequest` to the Miden node, and extracts an [AccountDetails] from
    /// the `GetAccountDetailsResponse` response.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// - There was an error sending the request to the node.
    /// - The answer had a `None` for one of the expected fields (account, summary, account_hash,
    ///   details).
    /// - There is an error during [Account] deserialization.
    async fn get_account_details(
        &mut self,
        account_id: AccountId,
    ) -> Result<AccountDetails, RpcError> {
        let request = GetAccountDetailsRequest { account_id: Some(account_id.into()) };

        let rpc_api = self.rpc_api().await?;

        let response = rpc_api.get_account_details(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::GetAccountDetails.to_string(),
                err.to_string(),
            )
        })?;
        let response = response.into_inner();
        let account_info = response.details.ok_or(RpcError::ExpectedDataMissing(
            "GetAccountDetails response should have an `account`".to_string(),
        ))?;

        let account_summary = account_info.summary.ok_or(RpcError::ExpectedDataMissing(
            "GetAccountDetails response's account should have a `summary`".to_string(),
        ))?;

        let hash = account_summary.account_hash.ok_or(RpcError::ExpectedDataMissing(
            "GetAccountDetails response's account should have an `account_hash`".to_string(),
        ))?;

        let hash = hash.try_into()?;

        let update_summary = AccountUpdateSummary::new(hash, account_summary.block_num);
        if account_id.is_public() {
            let details_bytes = account_info.details.ok_or(RpcError::ExpectedDataMissing(
                "GetAccountDetails response's account should have `details`".to_string(),
            ))?;

            let account = Account::read_from_bytes(&details_bytes)?;

            Ok(AccountDetails::Public(account, update_summary))
        } else {
            Ok(AccountDetails::Private(account_id, update_summary))
        }
    }

    /// Sends a `GetAccountProofs` request to the Miden node, and extracts a list of [AccountProof]
    /// from the response, as well as the block number that they were retrieved for.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// - One of the requested Accounts isn't returned by the node.
    /// - There was an error sending the request to the node.
    /// - The answer had a `None` for one of the expected fields.
    /// - There is an error during storage deserialization.
    async fn get_account_proofs(
        &mut self,
        account_requests: &BTreeSet<ForeignAccount>,
        known_account_codes: Vec<AccountCode>,
    ) -> Result<AccountProofs, RpcError> {
        let requested_accounts = account_requests.len();
        let mut rpc_account_requests: Vec<get_account_proofs_request::AccountRequest> =
            Vec::with_capacity(account_requests.len());

        for foreign_account in account_requests {
            rpc_account_requests.push(get_account_proofs_request::AccountRequest {
                account_id: Some(foreign_account.account_id().into()),
                storage_requests: foreign_account.storage_slot_requirements().into(),
            });
        }

        let known_account_codes: BTreeMap<Digest, AccountCode> =
            known_account_codes.into_iter().map(|c| (c.commitment(), c)).collect();

        let request = GetAccountProofsRequest {
            account_requests: rpc_account_requests,
            include_headers: Some(true),
            code_commitments: known_account_codes.keys().map(Into::into).collect(),
        };

        let rpc_api = self.rpc_api().await?;
        let response = rpc_api
            .get_account_proofs(request)
            .await
            .map_err(|err| {
                RpcError::RequestError(
                    NodeRpcClientEndpoint::GetAccountProofs.to_string(),
                    err.to_string(),
                )
            })?
            .into_inner();

        let mut account_proofs = Vec::with_capacity(response.account_proofs.len());
        let block_num = response.block_num.into();

        // sanity check response
        if requested_accounts != response.account_proofs.len() {
            return Err(RpcError::ExpectedDataMissing(
                "AccountProof did not contain all account IDs".to_string(),
            ));
        }

        for account in response.account_proofs {
            let merkle_proof = account
                .account_proof
                .ok_or(RpcError::ExpectedDataMissing("AccountProof".to_string()))?
                .try_into()?;

            let account_hash = account
                .account_hash
                .ok_or(RpcError::ExpectedDataMissing("AccountHash".to_string()))?
                .try_into()?;

            let account_id: AccountId = account
                .account_id
                .ok_or(RpcError::ExpectedDataMissing("AccountId".to_string()))?
                .try_into()?;

            // Because we set `include_headers` to true, for any public account we requeted we
            // should have the corresponding `state_header` field
            let headers = if account_id.is_public() {
                Some(
                    account
                        .state_header
                        .ok_or(RpcError::ExpectedDataMissing("Account.StateHeader".to_string()))?
                        .into_domain(account_id, &known_account_codes)?,
                )
            } else {
                None
            };

            let proof = AccountProof::new(account_id, merkle_proof, account_hash, headers)
                .map_err(|err| RpcError::InvalidResponse(err.to_string()))?;
            account_proofs.push(proof);
        }

        Ok((block_num, account_proofs))
    }

    /// Sends a `SyncNoteRequest` to the Miden node, and extracts a [NoteSyncInfo] from the
    /// response.
    async fn sync_notes(
        &mut self,
        block_num: BlockNumber,
        note_tags: &[NoteTag],
    ) -> Result<NoteSyncInfo, RpcError> {
        let note_tags = note_tags.iter().map(|&note_tag| note_tag.into()).collect();

        let request = SyncNoteRequest { block_num: block_num.as_u32(), note_tags };

        let rpc_api = self.rpc_api().await?;

        let response = rpc_api.sync_notes(request).await.map_err(|err| {
            RpcError::RequestError(NodeRpcClientEndpoint::SyncNotes.to_string(), err.to_string())
        })?;

        response.into_inner().try_into()
    }

    async fn check_nullifiers_by_prefix(
        &mut self,
        prefixes: &[u16],
        block_num: BlockNumber,
    ) -> Result<Vec<NullifierUpdate>, RpcError> {
        let request = CheckNullifiersByPrefixRequest {
            nullifiers: prefixes.iter().map(|&x| u32::from(x)).collect(),
            prefix_len: 16,
            block_num: block_num.as_u32(),
        };
        let rpc_api = self.rpc_api().await?;
        let response = rpc_api.check_nullifiers_by_prefix(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::CheckNullifiersByPrefix.to_string(),
                err.to_string(),
            )
        })?;
        let response = response.into_inner();
        let nullifiers = response
            .nullifiers
            .iter()
            .map(TryFrom::try_from)
            .collect::<Result<Vec<NullifierUpdate>, _>>()
            .map_err(|err| RpcError::InvalidResponse(err.to_string()))?;

        Ok(nullifiers)
    }

    async fn check_nullifiers(
        &mut self,
        nullifiers: &[Nullifier],
    ) -> Result<Vec<SmtProof>, RpcError> {
        let request = CheckNullifiersRequest {
            nullifiers: nullifiers.iter().map(|nul| nul.inner().into()).collect(),
        };
        let rpc_api = self.rpc_api().await?;

        let response = rpc_api.check_nullifiers(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::CheckNullifiers.to_string(),
                err.to_string(),
            )
        })?;

        let response = response.into_inner();
        let proofs = response.proofs.iter().map(TryInto::try_into).collect::<Result<_, _>>()?;

        Ok(proofs)
    }

    async fn get_account_state_delta(
        &mut self,
        account_id: AccountId,
        from_block: BlockNumber,
        to_block: BlockNumber,
    ) -> Result<AccountDelta, RpcError> {
        let request = GetAccountStateDeltaRequest {
            account_id: Some(account_id.into()),
            from_block_num: from_block.as_u32(),
            to_block_num: to_block.as_u32(),
        };

        let rpc_api = self.rpc_api().await?;
        let response = rpc_api.get_account_state_delta(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::GetAccountStateDelta.to_string(),
                err.to_string(),
            )
        })?;

        let response = response.into_inner();
        let delta = AccountDelta::read_from_bytes(&response.delta.ok_or(
            RpcError::ExpectedDataMissing("GetAccountStateDeltaResponse.delta".to_string()),
        )?)?;

        Ok(delta)
    }

    async fn get_block_by_number(
        &mut self,
        block_num: BlockNumber,
    ) -> Result<ProvenBlock, RpcError> {
        let request = GetBlockByNumberRequest { block_num: block_num.as_u32() };

        let rpc_api = self.rpc_api().await?;
        let response = rpc_api.get_block_by_number(request).await.map_err(|err| {
            RpcError::RequestError(
                NodeRpcClientEndpoint::GetBlockByNumber.to_string(),
                err.to_string(),
            )
        })?;

        let response = response.into_inner();
        let block =
            ProvenBlock::read_from_bytes(&response.block.ok_or(RpcError::ExpectedDataMissing(
                "GetBlockByNumberResponse.block".to_string(),
            ))?)?;

        Ok(block)
    }
}
