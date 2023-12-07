use core::fmt;
use crypto::StarkField;
use miden_lib::notes::{create_note, Script};
use miden_node_proto::{
    account_id::AccountId as ProtoAccountId,
    requests::{SubmitProvenTransactionRequest, SyncStateRequest},
    responses::{SubmitProvenTransactionResponse, SyncStateResponse},
};
use miden_tx::{DataStore, ProvingOptions, TransactionExecutor, TransactionProver};
use objects::{
    accounts::{Account, AccountId, AccountStub},
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, FungibleAsset},
    notes::{Note, RecordedNote},
    transaction::{ProvenTransaction, TransactionResult, TransactionScript, TransactionWitness},
    utils::collections::BTreeMap,
    Digest, Felt, Word,
};

use std::path::PathBuf;

mod store;
pub use store::AuthInfo;
pub use store::{mock_executor_data_store::MockDataStore, InputNoteFilter, Store};

#[cfg(any(test, feature = "testing"))]
pub mod mock;

pub mod errors;
use errors::ClientError;

// CONSTANTS
// ================================================================================================

/// The number of bits to shift identifiers for in use of filters.
pub const FILTER_ID_SHIFT: u8 = 48;
// TODO: How does this fit here considering miden-base? Can we construct it from parts?
pub struct ExecutedTransactionStub {
    pub id: String,
    pub account_id: u64,
    pub init_account_state: Digest,
    pub final_account_state: Digest,
    pub input_notes: Vec<RecordedNote>,
    pub output_notes: Vec<Note>,
    pub script_hash: Option<Digest>,
    pub script_program: Option<ProgramAst>,
    pub script_inputs: Option<BTreeMap<Digest, Vec<Felt>>>,
    pub block_num: u64,
    pub committed: bool,
    pub commit_height: u64,
}

impl ExecutedTransactionStub {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        account_id: u64,
        init_account_state: Digest,
        final_account_state: Digest,
        input_notes: Vec<RecordedNote>,
        output_notes: Vec<Note>,
        script_hash: Option<Digest>,
        script_program: Option<ProgramAst>,
        script_inputs: Option<BTreeMap<Digest, Vec<Felt>>>,
        block_num: u64,
        committed: bool,
        commit_height: u64,
    ) -> ExecutedTransactionStub {
        ExecutedTransactionStub {
            id,
            account_id,
            init_account_state,
            final_account_state,
            input_notes,
            output_notes,
            script_hash,
            script_program,
            script_inputs,
            block_num,
            committed,
            commit_height,
        }
    }
}

// MIDEN CLIENT
// ================================================================================================

/// A light client for connecting to the Miden rollup network.
///
/// Miden client is responsible for managing a set of accounts. Specifically, the client:
/// - Keeps track of the current and historical states of a set of accounts and related objects
///   such as notes and transactions.
/// - Connects to one or more Miden nodes to periodically sync with the current state of the
///   network.
/// - Executes, proves, and submits transactions to the network as directed by the user.
pub struct Client<D: DataStore> {
    /// Local database containing information about the accounts managed by this client.
    store: Store,
    #[cfg(not(any(test, feature = "testing")))]
    /// Api client for interacting with the Miden node.
    rpc_api: miden_node_proto::rpc::api_client::ApiClient<tonic::transport::Channel>,
    #[cfg(any(test, feature = "testing"))]
    pub rpc_api: mock::MockRpcApi,
    pub tx_executor: TransactionExecutor<D>,
}

impl<D: DataStore> Client<D> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client] instantiated with the specified configuration options.
    ///
    /// # Errors
    /// Returns an error if the client could not be instantiated.
    pub async fn new(
        config: ClientConfig,
        transaction_executor: TransactionExecutor<D>,
    ) -> Result<Self, ClientError> {
        Ok(Self {
            store: Store::new((&config).into())?,
            #[cfg(not(any(test, feature = "testing")))]
            rpc_api: miden_node_proto::rpc::api_client::ApiClient::connect(
                config.node_endpoint.to_string(),
            )
            .await
            .map_err(|err| ClientError::RpcApiError(errors::RpcApiError::ConnectionError(err)))?,
            #[cfg(any(test, feature = "testing"))]
            rpc_api: Default::default(),
            tx_executor: transaction_executor,
        })
    }

    // ACCOUNT INSERTION
    // --------------------------------------------------------------------------------------------

    /// Inserts a new account into the client's store.
    pub fn insert_account(
        &mut self,
        account: &Account,
        auth_info: &AuthInfo,
    ) -> Result<(), ClientError> {
        self.store
            .insert_account(account, auth_info)
            .map_err(ClientError::StoreError)
    }

    // ACCOUNT DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns summary info about the accounts managed by this client.
    ///
    /// TODO: replace `AccountStub` with a more relevant structure.
    pub fn get_accounts(&self) -> Result<Vec<AccountStub>, ClientError> {
        self.store.get_accounts().map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    pub fn get_account_by_id(&self, account_id: AccountId) -> Result<AccountStub, ClientError> {
        self.store
            .get_account_by_id(account_id)
            .map_err(|err| err.into())
    }

    /// Returns auth infor for an Account Id.
    pub fn get_account_auth(&self, account_id: AccountId) -> Result<AuthInfo, ClientError> {
        self.store
            .get_account_auth(account_id)
            .map_err(|err| err.into())
    }

    /// Returns vault assets from a vault root.
    pub fn get_vault_assets(&self, vault_root: Digest) -> Result<Vec<Asset>, ClientError> {
        self.store
            .get_vault_assets(vault_root)
            .map_err(|err| err.into())
    }

    /// Returns account code data from a root.
    pub fn get_account_code(
        &self,
        code_root: Digest,
    ) -> Result<(Vec<Digest>, ModuleAst), ClientError> {
        self.store
            .get_account_code(code_root)
            .map_err(|err| err.into())
    }

    /// Returns account storage data from a storage root.
    pub fn get_account_storage(
        &self,
        storage_root: Digest,
    ) -> Result<BTreeMap<u64, Word>, ClientError> {
        self.store
            .get_account_storage(storage_root)
            .map_err(|err| err.into())
    }

    /// Returns historical states for the account with the specified ID.
    ///
    /// TODO: wrap `Account` in a type with additional info.
    /// TODO: consider changing the interface to support pagination.
    pub fn get_account_history(&self, _account_id: AccountId) -> Result<Vec<Account>, ClientError> {
        todo!()
    }

    /// Returns detailed information about the current state of the account with the specified ID.
    ///
    /// TODO: wrap `Account` in a type with additional info (e.g., status).
    /// TODO: consider adding `nonce` as another parameter to identify a specific account state.
    pub fn get_account_details(&self, _account_id: AccountId) -> Result<Account, ClientError> {
        todo!()
    }

    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_input_notes(
        &self,
        filter: InputNoteFilter,
    ) -> Result<Vec<RecordedNote>, ClientError> {
        self.store.get_input_notes(filter).map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(&self, hash: Digest) -> Result<RecordedNote, ClientError> {
        self.store
            .get_input_note_by_hash(hash)
            .map_err(|err| err.into())
    }

    /// Returns unspent input notes managed by this client.
    pub fn get_unspent_input_notes(&self) -> Result<Vec<RecordedNote>, ClientError> {
        self.store
            .get_input_notes(InputNoteFilter::Committed)
            .map_err(|err| err.into())
    }

    /// Returns consumed input notes managed by this client.
    pub fn get_consumed_input_notes(&self) -> Result<Vec<RecordedNote>, ClientError> {
        self.store
            .get_input_notes(InputNoteFilter::Consumed)
            .map_err(|err| err.into())
    }

    /// Returns pending input notes managed by this client.
    pub fn get_pending_input_notes(&self) -> Result<Vec<RecordedNote>, ClientError> {
        self.store
            .get_input_notes(InputNoteFilter::Pending)
            .map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Inserts a new input note into the client's store.
    pub fn insert_input_note(&mut self, note: RecordedNote) -> Result<(), ClientError> {
        self.store
            .insert_input_note(&note)
            .map_err(|err| err.into())
    }

    // SYNC STATE
    // --------------------------------------------------------------------------------------------

    /// Returns the block number of the last state sync block
    pub fn get_latest_block_number(&self) -> Result<u32, ClientError> {
        self.store
            .get_latest_block_number()
            .map_err(|err| err.into())
    }

    /// Returns the list of note tags tracked by the client.
    pub fn get_note_tags(&self) -> Result<Vec<u64>, ClientError> {
        self.store.get_note_tags().map_err(|err| err.into())
    }

    /// Adds a note tag for the client to track.
    pub fn add_note_tag(&mut self, tag: u64) -> Result<(), ClientError> {
        self.store.add_note_tag(tag).map_err(|err| err.into())
    }

    /// Syncs the client's state with the current state of the Miden network.
    ///
    /// Returns the block number the client has been synced to.
    pub async fn sync_state(&mut self) -> Result<u32, ClientError> {
        let block_num = self.store.get_latest_block_number()?;
        let account_ids = self.store.get_account_ids()?;
        let note_tags = self.store.get_note_tags()?;
        let nullifiers = self.store.get_unspent_input_note_nullifiers()?;

        let response = self
            .sync_state_request(block_num, &account_ids, &note_tags, &nullifiers)
            .await?;

        let new_block_num = response.chain_tip;
        let new_nullifiers = response
            .nullifiers
            .into_iter()
            .filter_map(|x| {
                let nullifier = x.nullifier.as_ref().unwrap().try_into().unwrap();
                if nullifiers.contains(&nullifier) {
                    Some(nullifier)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        self.store
            .apply_state_sync(new_block_num, new_nullifiers)
            .map_err(ClientError::StoreError)?;

        Ok(new_block_num)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------
    /// Sends a sync state request to the Miden node and returns the response.
    async fn sync_state_request(
        &mut self,
        block_num: u32,
        account_ids: &[AccountId],
        note_tags: &[u64],
        nullifiers: &[Digest],
    ) -> Result<SyncStateResponse, ClientError> {
        let account_ids = account_ids
            .iter()
            .map(|id| ProtoAccountId { id: u64::from(*id) })
            .collect();
        let nullifiers = nullifiers
            .iter()
            .map(|nullifier| (nullifier[3].as_int() >> FILTER_ID_SHIFT) as u32)
            .collect();
        let note_tags = note_tags
            .iter()
            .map(|tag| (tag >> FILTER_ID_SHIFT) as u32)
            .collect::<Vec<_>>();

        let request = SyncStateRequest {
            block_num,
            account_ids,
            note_tags,
            nullifiers,
        };

        Ok(self
            .rpc_api
            .sync_state(request)
            .await
            .map_err(|err| ClientError::RpcApiError(errors::RpcApiError::RequestError(err)))?
            .into_inner())
    }

    async fn submit_proven_transaction_request(
        &mut self,
        _proven_transaction: ProvenTransaction,
    ) -> Result<SubmitProvenTransactionResponse, ClientError> {
        let request = SubmitProvenTransactionRequest {};

        Ok(self
            .rpc_api
            .submit_proven_transaction(request)
            .await
            .map_err(|err| ClientError::RpcApiError(errors::RpcApiError::RequestError(err)))?
            .into_inner())
    }

    // TRANSACTION CREATION
    // --------------------------------------------------------------------------------------------

    /// Inserts a new transaction into the client's store.
    fn insert_transaction(
        &mut self,
        transaction: &ProvenTransaction,
        transaction_script: Option<TransactionScript>,
    ) -> Result<(), ClientError> {
        self.store
            .insert_transaction(transaction, transaction_script)
            .map_err(|err| err.into())
    }

    // TRANSACTION DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_transactions(&self) -> Result<Vec<ExecutedTransactionStub>, ClientError> {
        self.store.get_transactions().map_err(|err| err.into())
    }

    // TRANSACTION
    // --------------------------------------------------------------------------------------------

    /// Creates and executes a transactions specified by the template, but does not change the
    /// local database.
    pub fn new_transaction(
        &self,
        transaction_template: TransactionTemplate,
    ) -> Result<TransactionResult, ClientError> {
        match transaction_template {
            TransactionTemplate::PayToId(PaymentTransaction {
                faucet_id,
                sender_account_id,
                target_account_id,
                amount,
            }) => {
                // Create assets
                let fungible_asset: Asset = FungibleAsset::new(faucet_id, amount).unwrap().into();
                let (target_pub_key, target_sk_pk_felt) =
                    store::mock_executor_data_store::get_new_key_pair_with_advice_map();
                let target_account =
                    store::mock_executor_data_store::get_account_with_default_account_code(
                        target_account_id,
                        target_pub_key,
                        None,
                    );

                // Create the note
                let p2id_script = Script::P2ID {
                    target: target_account_id,
                };
                let note = create_note(
                    p2id_script,
                    vec![fungible_asset],
                    sender_account_id,
                    None,
                    [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)], // TODO: Does this need to be random?
                )
                .map_err(ClientError::NoteError)?;

                let data_store: MockDataStore = MockDataStore::with_existing(
                    Some(target_account.clone()),
                    Some(vec![note.clone()]),
                );

                let mut executor = TransactionExecutor::new(data_store.clone());
                executor.load_account(target_account_id).unwrap();

                let block_ref = data_store.block_header.block_num().as_int() as u32;
                let note_origins = data_store
                    .notes
                    .iter()
                    .map(|note| note.origin().clone())
                    .collect::<Vec<_>>();

                let tx_script_code = ProgramAst::parse(
                    "
                    use.miden::eoa::basic->auth_tx

                    begin
                        call.auth_tx::auth_tx_rpo_falcon512
                    end
                    "
                    .to_string()
                    .as_str(),
                )
                .unwrap();

                let tx_script_target = executor
                    .compile_tx_script(
                        tx_script_code.clone(),
                        vec![(target_pub_key, target_sk_pk_felt)],
                        vec![],
                    )
                    .map_err(ClientError::ExecutorError)?;

                // Execute the transaction and get the witness
                let transaction_result = executor
                    .execute_transaction(
                        target_account_id,
                        block_ref,
                        &note_origins,
                        Some(tx_script_target),
                    )
                    .map_err(ClientError::ExecutorError)?;

                Ok(transaction_result)
            }
            TransactionTemplate::PayToIdWithRecall(_) => todo!(),
            TransactionTemplate::ConsumeNotes(_) => todo!(),
        }
    }

    /// Proves the specified transaction witness, submits it to the node, and saves the transaction into
    /// the local database.
    pub async fn send_transaction(
        &mut self,
        transaction_witness: TransactionWitness,
        transaction_script: Option<TransactionScript>,
    ) -> Result<(), ClientError> {
        let transaction_prover = TransactionProver::new(ProvingOptions::default());
        let proven_transaction = transaction_prover
            .prove_transaction_witness(transaction_witness)
            .map_err(ClientError::TransactionProverError)?;

        self.submit_proven_transaction_request(proven_transaction.clone())
            .await?;

        self.insert_transaction(&proven_transaction, transaction_script)?;
        Ok(())
    }
}

pub enum TransactionTemplate {
    /// Creates a pay-to-id note directed to a specific account from a faucet
    PayToId(PaymentTransaction),
    /// Creates a pay-to-id note directed to a specific account with a recall timeout
    PayToIdWithRecall(PaymentTransaction),
    /// Consume all outstanding notes for an account
    ConsumeNotes(AccountId),
}

pub struct PaymentTransaction {
    faucet_id: AccountId,
    sender_account_id: AccountId,
    target_account_id: AccountId,
    amount: u64,
}

// CLIENT CONFIG
// ================================================================================================

/// Configuration options of Miden client.
#[derive(Debug, PartialEq, Eq)]
pub struct ClientConfig {
    /// Location of the client's data file.
    store_path: String,
    /// Address of the Miden node to connect to.
    node_endpoint: Endpoint,
}

impl ClientConfig {
    /// Returns a new instance of [ClientConfig] with the specified store path and node endpoint.
    pub fn new(store_path: String, node_endpoint: Endpoint) -> Self {
        Self {
            store_path,
            node_endpoint,
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        const STORE_FILENAME: &str = "store.sqlite3";

        // get directory of the currently executing binary, or fallback to the current directory
        let exec_dir = match std::env::current_exe() {
            Ok(mut path) => {
                path.pop();
                path
            }
            Err(_) => PathBuf::new(),
        };

        let store_path = exec_dir.join(STORE_FILENAME);

        Self {
            store_path: store_path
                .into_os_string()
                .into_string()
                .expect("Creating the hardcoded path to the store file should not panic"),
            node_endpoint: Endpoint::default(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Endpoint {
    protocol: String,
    host: String,
    port: u16,
}

impl Endpoint {
    /// Returns a new instance of [Endpoint] with the specified protocol, host, and port.
    pub fn new(protocol: String, host: String, port: u16) -> Self {
        Self {
            protocol,
            host,
            port,
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}:{}", self.protocol, self.host, self.port)
    }
}

impl Default for Endpoint {
    fn default() -> Self {
        const MIDEN_NODE_PORT: u16 = 57291;

        Self {
            protocol: "http".to_string(),
            host: "localhost".to_string(),
            port: MIDEN_NODE_PORT,
        }
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use crate::store::{mock_executor_data_store::MockDataStore, AuthInfo};

    use super::store::tests::create_test_store_path;
    use crypto::dsa::rpo_falcon512::KeyPair;
    use miden_lib::assembler::assembler;
    use miden_tx::TransactionExecutor;
    use mock::mock::{
        account::{self, MockAccountType},
        notes::AssetPreservationStatus,
        transaction::mock_inputs,
    };

    #[tokio::test]
    async fn test_input_notes_round_trip() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = super::Client::new(
            super::ClientConfig::new(
                store_path.into_os_string().into_string().unwrap(),
                super::Endpoint::default(),
            ),
            TransactionExecutor::new(MockDataStore::default()),
        )
        .await
        .unwrap();

        // generate test data
        let (_, _, _, recorded_notes) = mock_inputs(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
        );

        // insert notes into database
        for note in recorded_notes.iter().cloned() {
            client.insert_input_note(note).unwrap();
        }

        // retrieve notes from database
        let retrieved_notes = client.get_input_notes(crate::InputNoteFilter::All).unwrap();

        // compare notes
        assert_eq!(recorded_notes, retrieved_notes);
    }

    // #[tokio::test]
    // async fn test_submit_proven_tx() {
    //     let store_path = create_test_store_path();

    //     // generate test client
    //     let mut client = super::Client::new(
    //         super::ClientConfig::new(
    //             store_path.into_os_string().into_string().unwrap(),
    //             super::Endpoint::default(),
    //         ),
    //         TransactionExecutor::new(MockDataStore::default()),
    //     )
    //     .await
    //     .unwrap();

    //     client.new_transaction(crate::TransactionTemplate::PayToId(
    //         crate::PaymentTransaction { faucet_id: (), sender_account_id: (), target_account_id: (), amount: () }
    //     ));
    // }

    #[tokio::test]
    async fn test_get_input_note() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = super::Client::new(
            super::ClientConfig::new(
                store_path.into_os_string().into_string().unwrap(),
                super::Endpoint::default(),
            ),
            TransactionExecutor::new(MockDataStore::default()),
        )
        .await
        .unwrap();

        // generate test data
        let (_, _, _, recorded_notes) = mock_inputs(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
        );

        // insert note into database
        client.insert_input_note(recorded_notes[0].clone()).unwrap();

        // retrieve note from database
        let retrieved_note = client
            .get_input_note(recorded_notes[0].note().hash())
            .unwrap();

        // compare notes
        assert_eq!(recorded_notes[0], retrieved_note);
    }

    #[tokio::test]
    async fn insert_same_account_twice_fails() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = super::Client::new(
            super::ClientConfig::new(
                store_path.into_os_string().into_string().unwrap(),
                super::Endpoint::default(),
            ),
            TransactionExecutor::new(MockDataStore::default()),
        )
        .await
        .unwrap();

        let assembler = assembler();
        let account = account::mock_new_account(&assembler);

        let key_pair: KeyPair = KeyPair::new()
            .map_err(|err| format!("Error generating KeyPair: {}", err))
            .unwrap();

        assert!(client
            .insert_account(&account, &AuthInfo::RpoFalcon512(key_pair))
            .is_ok());
        assert!(client
            .insert_account(&account, &AuthInfo::RpoFalcon512(key_pair))
            .is_err());
    }

    #[tokio::test]
    async fn test_sync_state() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = super::Client::new(
            super::ClientConfig::new(
                store_path.into_os_string().into_string().unwrap(),
                super::Endpoint::default(),
            ),
            TransactionExecutor::new(MockDataStore::default()),
        )
        .await
        .unwrap();

        // generate test data
        crate::mock::insert_mock_data(&mut client);

        // assert that we have no consumed notes prior to syncing state
        assert_eq!(
            client
                .get_input_notes(crate::InputNoteFilter::Consumed)
                .unwrap()
                .len(),
            0
        );

        // sync state
        let block_num = client.sync_state().await.unwrap();

        // verify that the client is synced to the latest block
        assert_eq!(
            block_num,
            client
                .rpc_api
                .sync_state_requests
                .first_key_value()
                .unwrap()
                .1
                .chain_tip
        );

        // verify that we now have one consumed note after syncing state
        assert_eq!(
            client
                .get_input_notes(crate::InputNoteFilter::Consumed)
                .unwrap()
                .len(),
            1
        );

        // verify that the latest block number has been updated
        assert_eq!(
            client.get_latest_block_number().unwrap(),
            client
                .rpc_api
                .sync_state_requests
                .first_key_value()
                .unwrap()
                .1
                .chain_tip
        );
    }

    #[tokio::test]
    async fn test_add_tag() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = super::Client::new(
            super::ClientConfig::new(
                store_path.into_os_string().into_string().unwrap(),
                super::Endpoint::default(),
            ),
            TransactionExecutor::new(MockDataStore::default()),
        )
        .await
        .unwrap();

        // assert that no tags are being tracked
        assert_eq!(client.get_note_tags().unwrap().len(), 0);

        // add a tag
        const TAG_VALUE_1: u64 = 1;
        const TAG_VALUE_2: u64 = 2;
        client.add_note_tag(TAG_VALUE_1).unwrap();
        client.add_note_tag(TAG_VALUE_2).unwrap();

        // verify that the tag is being tracked
        assert_eq!(
            client.get_note_tags().unwrap(),
            vec![TAG_VALUE_1, TAG_VALUE_2]
        );
    }
}
