use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use miden_lib::notes::{create_p2id_note, create_p2idr_note, create_swap_note};
use miden_objects::{
    accounts::{AccountDelta, AccountId, AccountType},
    assembly::ProgramAst,
    assets::{
        Asset::{Fungible, NonFungible},
        FungibleAsset, NonFungibleAsset
    },
    notes::{Note, NoteDetails, NoteExecutionMode, NoteId, NoteTag, NoteType},
    transaction::{InputNotes, TransactionArgs},
    AssetError, Digest, Felt, FieldElement, NoteError, Word,
};
use miden_tx::{auth::TransactionAuthenticator, ProvingOptions, TransactionProver};
use request::{TransactionRequestError, TransactionScriptTemplate};
use script_builder::{AccountCapabilities, AccountInterface, TransactionScriptBuilder};
use tracing::info;
use winter_maybe_async::{maybe_async, maybe_await};

use self::request::{
    PaymentTransactionData, SwapTransactionData, TransactionRequest, TransactionTemplate,
};
use super::{rpc::NodeRpcClient, Client, FeltRng};
use crate::{
    notes::NoteScreener,
    store::{InputNoteRecord, NoteFilter, Store, TransactionFilter},
    ClientError,
};

pub mod request;
pub mod script_builder;
pub use miden_objects::transaction::{
    ExecutedTransaction, InputNote, OutputNote, OutputNotes, ProvenTransaction, TransactionId,
    TransactionScript,
};
pub use miden_tx::{DataStoreError, ScriptTarget, TransactionExecutorError};
pub use request::known_script_roots;

// TRANSACTION RESULT
// --------------------------------------------------------------------------------------------

/// Represents the result of executing a transaction by the client.
///  
/// It contains an [ExecutedTransaction], and a list of `relevant_notes` that contains the
/// `output_notes` that the client has to store as input notes, based on the NoteScreener
/// output from filtering the transaction's output notes or some partial note we expect to receive
/// in the future (you can check at swap notes for an example of this).
#[derive(Clone, Debug)]
pub struct TransactionResult {
    transaction: ExecutedTransaction,
    relevant_notes: Vec<InputNoteRecord>,
}

impl TransactionResult {
    /// Screens the output notes to store and track the relevant ones, and instantiates a [TransactionResult]
    #[maybe_async]
    pub fn new<S: Store>(
        transaction: ExecutedTransaction,
        note_screener: NoteScreener<S>,
        partial_notes: Vec<NoteDetails>,
    ) -> Result<Self, ClientError> {
        let mut relevant_notes = vec![];

        for note in notes_from_output(transaction.output_notes()) {
            let account_relevance = maybe_await!(note_screener.check_relevance(note))?;

            if !account_relevance.is_empty() {
                relevant_notes.push(note.clone().into());
            }
        }

        // Include partial output notes into the relevant notes
        relevant_notes.extend(partial_notes.iter().map(InputNoteRecord::from));

        let tx_result = Self { transaction, relevant_notes };

        Ok(tx_result)
    }

    pub fn executed_transaction(&self) -> &ExecutedTransaction {
        &self.transaction
    }

    pub fn created_notes(&self) -> &OutputNotes {
        self.transaction.output_notes()
    }

    pub fn relevant_notes(&self) -> &[InputNoteRecord] {
        &self.relevant_notes
    }

    pub fn block_num(&self) -> u32 {
        self.transaction.block_header().block_num()
    }

    pub fn transaction_arguments(&self) -> &TransactionArgs {
        self.transaction.tx_args()
    }

    pub fn account_delta(&self) -> &AccountDelta {
        self.transaction.account_delta()
    }

    pub fn consumed_notes(&self) -> &InputNotes<InputNote> {
        self.transaction.tx_inputs().input_notes()
    }
}

// TRANSACTION RECORD
// --------------------------------------------------------------------------------------------

/// Describes a transaction that has been executed and is being tracked on the Client
///
/// Currently, the `commit_height` (and `committed` status) is set based on the height
/// at which the transaction's output notes are committed.
#[derive(Debug)]
pub struct TransactionRecord {
    pub id: TransactionId,
    pub account_id: AccountId,
    pub init_account_state: Digest,
    pub final_account_state: Digest,
    pub input_note_nullifiers: Vec<Digest>,
    pub output_notes: OutputNotes,
    pub transaction_script: Option<TransactionScript>,
    pub block_num: u32,
    pub transaction_status: TransactionStatus,
}

impl TransactionRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TransactionId,
        account_id: AccountId,
        init_account_state: Digest,
        final_account_state: Digest,
        input_note_nullifiers: Vec<Digest>,
        output_notes: OutputNotes,
        transaction_script: Option<TransactionScript>,
        block_num: u32,
        transaction_status: TransactionStatus,
    ) -> TransactionRecord {
        TransactionRecord {
            id,
            account_id,
            init_account_state,
            final_account_state,
            input_note_nullifiers,
            output_notes,
            transaction_script,
            block_num,
            transaction_status,
        }
    }
}

/// Represents the status of a transaction
#[derive(Debug, Clone)]
pub enum TransactionStatus {
    /// Transaction has been submitted but not yet committed
    Pending,
    /// Transaction has been committed and included at the specified block number
    Committed(u32),
}

impl fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "Pending"),
            TransactionStatus::Committed(block_number) => {
                write!(f, "Committed (Block: {})", block_number)
            },
        }
    }
}

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    // TRANSACTION DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Retrieves tracked transactions, filtered by [TransactionFilter].
    #[maybe_async]
    pub fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, ClientError> {
        maybe_await!(self.store.get_transactions(filter)).map_err(|err| err.into())
    }

    // TRANSACTION
    // --------------------------------------------------------------------------------------------

    /// Compiles a [TransactionTemplate] into a [TransactionRequest] that can be then executed by the
    /// client
    #[maybe_async]
    pub fn build_transaction_request(
        &mut self,
        transaction_template: TransactionTemplate,
    ) -> Result<TransactionRequest, ClientError> {
        match transaction_template {
            TransactionTemplate::ConsumeNotes(account_id, notes) => {
                let notes = notes.iter().map(|id| (*id, None));

                let tx_request =
                    TransactionRequest::new(account_id).with_authenticated_input_notes(notes);

                Ok(tx_request)
            },
            TransactionTemplate::MintFungibleAsset(asset, target_account_id, note_type) => {
                self.build_mint_tx_request(asset, target_account_id, note_type)
            },
            TransactionTemplate::PayToId(payment_data, note_type) => {
                self.build_p2id_tx_request(payment_data, None, note_type)
            },
            TransactionTemplate::PayToIdWithRecall(payment_data, recall_height, note_type) => {
                self.build_p2id_tx_request(payment_data, Some(recall_height), note_type)
            },
            TransactionTemplate::Swap(swap_data, note_type) => {
                self.build_swap_tx_request(swap_data, note_type)
            },
        }
    }

    /// Creates and executes a transaction specified by the request, but does not change the
    /// local database.
    ///
    /// # Errors
    ///
    /// - Returns [ClientError::MissingOutputNotes] if the [TransactionRequest] ouput notes are
    ///   not a subset of executor's output notes
    /// - Returns a [ClientError::TransactionExecutorError] if the execution fails
    /// - Returns a [ClientError::TransactionRequestError] if the request is invalid
    #[maybe_async]
    pub fn new_transaction(
        &mut self,
        transaction_request: TransactionRequest,
    ) -> Result<TransactionResult, ClientError> {
        // Validates the transaction request before executing
        maybe_await!(self.validate_request(&transaction_request))?;

        let account_id = transaction_request.account_id();
        maybe_await!(self.tx_executor.load_account(account_id))
            .map_err(ClientError::TransactionExecutorError)?;

        // Ensure authenticated notes have their inclusion proofs (a.k.a they're in a committed
        // state). TODO: we should consider refactoring this in a way we can handle this in
        // `get_transaction_inputs`
        let authenticated_input_note_ids: Vec<NoteId> =
            transaction_request.authenticated_input_note_ids().collect::<Vec<_>>();

        let authenticated_note_records = maybe_await!(self
            .store
            .get_input_notes(NoteFilter::List(&authenticated_input_note_ids)))?;

        for authenticated_note_record in authenticated_note_records {
            if !authenticated_note_record.is_authenticated() {
                return Err(ClientError::TransactionRequestError(
                    TransactionRequestError::InputNoteNotAuthenticated,
                ));
            }
        }

        // If tx request contains unauthenticated_input_notes we should insert them
        for unauthenticated_input_note in transaction_request.unauthenticated_input_notes() {
            // TODO: run this as a single TX
            maybe_await!(self.store.insert_input_note(unauthenticated_input_note.clone().into()))?;
        }

        let block_num = maybe_await!(self.store.get_sync_height())?;

        let note_ids = transaction_request.get_input_note_ids();
        let output_notes: Vec<Note> =
            transaction_request.expected_output_notes().cloned().collect();
        let future_notes: Vec<NoteDetails> =
            transaction_request.expected_future_notes().cloned().collect();

        let tx_script = match transaction_request.script_template() {
            Some(TransactionScriptTemplate::CustomScript(script)) => script.clone(),
            Some(TransactionScriptTemplate::SendNotes(notes)) => {
                let tx_script_builder = TransactionScriptBuilder::new(maybe_await!(
                    self.get_account_capabilities(account_id)
                )?);

                tx_script_builder.build_send_notes_script(&self.tx_executor, notes)?
            },
            None => {
                if transaction_request.input_notes().is_empty() {
                    return Err(ClientError::TransactionRequestError(
                        TransactionRequestError::NoInputNotes,
                    ));
                }

                let tx_script_builder = TransactionScriptBuilder::new(maybe_await!(
                    self.get_account_capabilities(account_id)
                )?);

                tx_script_builder.build_auth_script(&self.tx_executor)?
            },
        };

        let tx_args = transaction_request.into_transaction_args(tx_script);

        // Execute the transaction and get the witness
        let executed_transaction = maybe_await!(self
            .tx_executor
            .execute_transaction(account_id, block_num, &note_ids, tx_args,))?;

        // Check that the expected output notes matches the transaction outcome.
        // We compare authentication hashes where possible since that involves note IDs + metadata
        // (as opposed to just note ID which remains the same regardless of metadata)
        // We also do the check for partial output notes
        let tx_note_auth_hashes: BTreeSet<Digest> =
            notes_from_output(executed_transaction.output_notes())
                .map(|note| note.hash())
                .collect();

        let missing_note_ids: Vec<NoteId> = output_notes
            .iter()
            .filter_map(|n| (!tx_note_auth_hashes.contains(&n.hash())).then_some(n.id()))
            .collect();

        if !missing_note_ids.is_empty() {
            return Err(ClientError::MissingOutputNotes(missing_note_ids));
        }

        let screener = NoteScreener::new(self.store.clone());

        maybe_await!(TransactionResult::new(executed_transaction, screener, future_notes))
    }

    /// Proves the specified transaction, submits it to the network, and saves the transaction into
    /// the local database for tracking.
    pub async fn submit_transaction(
        &mut self,
        tx_result: TransactionResult,
    ) -> Result<(), ClientError> {
        let transaction_prover = TransactionProver::new(ProvingOptions::default());

        info!("Proving transaction...");
        let proven_transaction =
            transaction_prover.prove_transaction(tx_result.executed_transaction().clone())?;
        info!("Transaction proven.");

        info!("Submitting transaction to the network...");
        self.rpc_api.submit_proven_transaction(proven_transaction).await?;
        info!("Transaction submitted.");

        // Transaction was proven and submitted to the node correctly, persist note details and update account
        info!("Applying transaction to the local store...");
        maybe_await!(self.store.apply_transaction(tx_result))?;
        info!("Transaction stored.");
        Ok(())
    }

    /// Compiles the provided transaction script source and inputs into a [TransactionScript] and
    /// checks (to the extent possible) that the transaction script can be executed against all
    /// accounts with the specified interfaces.
    pub fn compile_tx_script<T>(
        &self,
        program: ProgramAst,
        inputs: T,
        target_account_procs: Vec<ScriptTarget>,
    ) -> Result<TransactionScript, ClientError>
    where
        T: IntoIterator<Item = (Word, Vec<Felt>)>,
    {
        self.tx_executor
            .compile_tx_script(program, inputs, target_account_procs)
            .map_err(ClientError::TransactionExecutorError)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    fn calculate_outgoing_assets(
        &self,
        transaction_request: &TransactionRequest,
    ) -> (&BTreeMap<AccountId, u64>, BTreeSet<&NonFungibleAsset>) {
        // Get own notes assets
        let own_notes_assets = match transaction_request.script_template() {
            Some(TransactionScriptTemplate::SendNotes(notes)) => {
                notes.iter().map(|note| (note.id(), note.assets())).collect::<Vec<_>>()
            },
            _ => vec![],
        };

        // Get transaction output notes assets
        let mut output_notes_assets = transaction_request
            .expected_output_notes()
            .map(|note| (note.id(), note.assets()))
            .collect::<BTreeMap<_, _>>();
            
        // Merge with own notes assets and delete duplicates
        output_notes_assets
        .append(
            &mut BTreeMap::from_iter(
                own_notes_assets
                .into_iter()
            )
        );

        let mut fungible_balance_map: BTreeMap<AccountId, u64> = BTreeMap::new();
        let mut non_fungible_vec: BTreeSet<&NonFungibleAsset> = BTreeSet::new(); 

        // Create a map of the fungible and non-fungible assets in the output notes
        for asset in output_notes_assets.iter().flat_map(|(_, note_assets)| note_assets.iter()) {
            match asset {
                Fungible(fungible) => {
                    *fungible_balance_map.entry(fungible.faucet_id()).or_insert(0) +=
                        fungible.amount();
                },
                NonFungible(non_fungible) => {
                    non_fungible_vec.insert(non_fungible);
                },
            }
        }

        (&fungible_balance_map, non_fungible_vec)
    }

    #[maybe_async]
    fn validate_request(
        &self,
        transaction_request: &TransactionRequest,
    ) -> Result<(), ClientError> {
        let (fungible_balance_map, non_fungible_vec) =
            self.calculate_outgoing_assets(transaction_request);

        // Get client assets
        let (account, _) = maybe_await!(self.get_account(transaction_request.account_id()))?;
        let account_assets = account.vault().assets();

        // Check if the output notes assets are less than or equal to the account assets
        for account_asset in account_assets {
            match account_asset {
                Fungible(account_fungible) => {
                    if let Some(&required_amount) =
                        fungible_balance_map.get(&account_fungible.faucet_id())
                    {
                        if account_fungible.amount() < required_amount {
                            return Err(ClientError::AssetError(
                                AssetError::AssetAmountNotSufficient(
                                    account_fungible.amount(),
                                    required_amount,
                                ),
                            ));
                        }
                    }
                },
                NonFungible(account_non_fungible) => {
                    if !non_fungible_vec.contains(&&account_non_fungible) {
                        return Err(ClientError::AssetError(AssetError::AssetAmountNotSufficient(
                            1, 0,
                        )));
                    }
                },
            }
        }

        Ok(())
    }

    /// Helper to build a [TransactionRequest] for P2ID-type transactions easily.
    ///
    /// - auth_info has to be from the executor account
    /// - If recall_height is Some(), a P2IDR note will be created. Otherwise, a P2ID is created.
    fn build_p2id_tx_request(
        &mut self,
        payment_data: PaymentTransactionData,
        recall_height: Option<u32>,
        note_type: NoteType,
    ) -> Result<TransactionRequest, ClientError> {
        let created_note = if let Some(recall_height) = recall_height {
            create_p2idr_note(
                payment_data.account_id(),
                payment_data.target_account_id(),
                vec![payment_data.asset()],
                note_type,
                Felt::ZERO,
                recall_height,
                &mut self.rng,
            )?
        } else {
            create_p2id_note(
                payment_data.account_id(),
                payment_data.target_account_id(),
                vec![payment_data.asset()],
                note_type,
                Felt::ZERO,
                &mut self.rng,
            )?
        };

        Ok(TransactionRequest::new(payment_data.account_id())
            .with_own_output_notes(vec![OutputNote::Full(created_note)])?)
    }

    /// Helper to build a [TransactionRequest] for Swap-type transactions easily.
    ///
    /// - auth_info has to be from the executor account
    fn build_swap_tx_request(
        &mut self,
        swap_data: SwapTransactionData,
        note_type: NoteType,
    ) -> Result<TransactionRequest, ClientError> {
        // The created note is the one that we need as the output of the tx, the other one is the
        // one that we expect to receive and consume eventually
        let (created_note, payback_note_details) = create_swap_note(
            swap_data.account_id(),
            swap_data.offered_asset(),
            swap_data.requested_asset(),
            note_type,
            Felt::ZERO,
            &mut self.rng,
        )?;

        Ok(TransactionRequest::new(swap_data.account_id())
            .with_expected_future_notes(vec![payback_note_details])
            .with_own_output_notes(vec![OutputNote::Full(created_note)])?)
    }

    /// Helper to build a [TransactionRequest] for transaction to mint fungible tokens.
    ///
    /// - faucet_auth_info has to be from the faucet account
    fn build_mint_tx_request(
        &mut self,
        asset: FungibleAsset,
        target_account_id: AccountId,
        note_type: NoteType,
    ) -> Result<TransactionRequest, ClientError> {
        let created_note = create_p2id_note(
            asset.faucet_id(),
            target_account_id,
            vec![asset.into()],
            note_type,
            Felt::ZERO,
            &mut self.rng,
        )?;

        Ok(TransactionRequest::new(asset.faucet_id())
            .with_own_output_notes(vec![OutputNote::Full(created_note)])?)
    }

    /// Retrieves the account capabilities for the specified account.
    #[maybe_async]
    fn get_account_capabilities(
        &self,
        account_id: AccountId,
    ) -> Result<AccountCapabilities, ClientError> {
        let account = maybe_await!(self.get_account(account_id))?.0;
        let account_auth = maybe_await!(self.get_account_auth(account_id))?;

        // TODO: we should check if the account actually exposes the interfaces we're trying to use
        let account_capabilities = match account.account_type() {
            AccountType::FungibleFaucet => AccountInterface::BasicFungibleFaucet,
            AccountType::NonFungibleFaucet => todo!("Non fungible faucet not supported yet"),
            AccountType::RegularAccountImmutableCode | AccountType::RegularAccountUpdatableCode => {
                AccountInterface::BasicWallet
            },
        };

        Ok(AccountCapabilities {
            account_id,
            auth: account_auth,
            interfaces: account_capabilities,
        })
    }
}

// HELPERS
// ================================================================================================

pub(crate) fn prepare_word(word: &Word) -> String {
    word.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>().join(".")
}

/// Extracts notes from [OutputNotes]
/// Used for:
/// - checking the relevance of notes to save them as input notes
/// - validate hashes versus expected output notes after a transaction is executed
pub fn notes_from_output(output_notes: &OutputNotes) -> impl Iterator<Item = &Note> {
    output_notes
        .iter()
        .filter(|n| matches!(n, OutputNote::Full(_)))
        .map(|n| match n {
            OutputNote::Full(n) => n,
            // The following todo!() applies until we have a way to support flows where we have
            // partial details of the note
            OutputNote::Header(_) | OutputNote::Partial(_) => {
                todo!("For now, all details should be held in OutputNote::Fulls")
            },
        })
}

/// Returns a note tag for a swap note with the specified parameters.
///
/// Use case ID for the returned tag is set to 0.
///
/// Tag payload is constructed by taking asset tags (8 bits of faucet ID) and concatenating them
/// together as offered_asset_tag + requested_asset tag.
///
/// Network execution hint for the returned tag is set to `Local`.
///
/// Based on miden-base's implementation (<https://github.com/0xPolygonMiden/miden-base/blob/9e4de88031b55bcc3524cb0ccfb269821d97fb29/miden-lib/src/notes/mod.rs#L153>)
///
/// TODO: we should make the function in base public and once that gets released use that one and
/// delete this implementation.
pub fn build_swap_tag(
    note_type: NoteType,
    offered_asset_faucet_id: AccountId,
    requested_asset_faucet_id: AccountId,
) -> Result<NoteTag, NoteError> {
    const SWAP_USE_CASE_ID: u16 = 0;

    // get bits 4..12 from faucet IDs of both assets, these bits will form the tag payload; the
    // reason we skip the 4 most significant bits is that these encode metadata of underlying
    // faucets and are likely to be the same for many different faucets.

    let offered_asset_id: u64 = offered_asset_faucet_id.into();
    let offered_asset_tag = (offered_asset_id >> 52) as u8;

    let requested_asset_id: u64 = requested_asset_faucet_id.into();
    let requested_asset_tag = (requested_asset_id >> 52) as u8;

    let payload = ((offered_asset_tag as u16) << 8) | (requested_asset_tag as u16);

    let execution = NoteExecutionMode::Local;
    match note_type {
        NoteType::Public => NoteTag::for_public_use_case(SWAP_USE_CASE_ID, payload, execution),
        _ => NoteTag::for_local_use_case(SWAP_USE_CASE_ID, payload),
    }
}
