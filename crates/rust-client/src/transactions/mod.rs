//! Provides APIs for transaction creation, execution, and proving.
//! It also handles proof submission to the network.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountDelta, AccountId, AccountType},
    assets::{Asset, NonFungibleAsset},
    notes::{Note, NoteDetails, NoteExecutionMode, NoteId, NoteTag, NoteType},
    transaction::{InputNotes, TransactionArgs},
    AssetError, Digest, Felt, NoteError, Word,
};
pub use miden_tx::{LocalTransactionProver, ProvingOptions, TransactionProver};
use script_builder::{AccountCapabilities, AccountInterface};
use tracing::info;
use winter_maybe_async::*;

use super::{Client, FeltRng};
use crate::{
    notes::{NoteScreener, NoteUpdates},
    store::{
        input_note_states::ExpectedNoteState, InputNoteRecord, InputNoteState, NoteFilter,
        OutputNoteRecord, TransactionFilter,
    },
    sync::NoteTagRecord,
    ClientError,
};

mod request;
pub use request::{
    NoteArgs, PaymentTransactionData, SwapTransactionData, TransactionRequest,
    TransactionRequestError, TransactionScriptTemplate,
};

mod script_builder;
pub use miden_objects::transaction::{
    ExecutedTransaction, InputNote, OutputNote, OutputNotes, ProvenTransaction, TransactionId,
    TransactionScript,
};
pub use miden_tx::{DataStoreError, TransactionExecutorError};
pub use script_builder::TransactionScriptBuilderError;

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
    /// Screens the output notes to store and track the relevant ones, and instantiates a
    /// [TransactionResult]
    #[maybe_async]
    pub fn new(
        transaction: ExecutedTransaction,
        note_screener: NoteScreener,
        partial_notes: Vec<(NoteDetails, NoteTag)>,
    ) -> Result<Self, ClientError> {
        let mut relevant_notes = vec![];

        for note in notes_from_output(transaction.output_notes()) {
            let account_relevance = maybe_await!(note_screener.check_relevance(note))?;

            if !account_relevance.is_empty() {
                relevant_notes.push(note.clone().into());
            }
        }

        // Include partial output notes into the relevant notes
        relevant_notes.extend(partial_notes.iter().map(|(note_details, tag)| {
            InputNoteRecord::new(
                note_details.clone(),
                None,
                ExpectedNoteState {
                    metadata: None,
                    after_block_num: 0,
                    tag: Some(*tag),
                }
                .into(),
            )
        }));

        let tx_result = Self { transaction, relevant_notes };

        Ok(tx_result)
    }

    /// Returns the [ExecutedTransaction].
    pub fn executed_transaction(&self) -> &ExecutedTransaction {
        &self.transaction
    }

    /// Returns the output notes that were generated as a result of the transaction execution.
    pub fn created_notes(&self) -> &OutputNotes {
        self.transaction.output_notes()
    }

    /// Returns the list of notes that are relevant to the client, based on [NoteScreener].
    pub fn relevant_notes(&self) -> &[InputNoteRecord] {
        &self.relevant_notes
    }

    /// Returns the block against which the transaction was executed.
    pub fn block_num(&self) -> u32 {
        self.transaction.block_header().block_num()
    }

    /// Returns transaction's [TransactionArgs].
    pub fn transaction_arguments(&self) -> &TransactionArgs {
        self.transaction.tx_args()
    }

    /// Returns the [AccountDelta] that describes the change of state for the executing [Account].
    pub fn account_delta(&self) -> &AccountDelta {
        self.transaction.account_delta()
    }

    /// Returns input notes that were consumed as part of the transaction.
    pub fn consumed_notes(&self) -> &InputNotes<InputNote> {
        self.transaction.tx_inputs().input_notes()
    }
}

impl From<TransactionResult> for ExecutedTransaction {
    fn from(tx_result: TransactionResult) -> ExecutedTransaction {
        tx_result.transaction
    }
}

// TRANSACTION RECORD
// --------------------------------------------------------------------------------------------

/// Describes a transaction that has been executed and is being tracked on the Client
///
/// Currently, the `commit_height` (and `committed` status) is set based on the height
/// at which the transaction's output notes are committed.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    /// Transaction has been submitted but not yet committed
    Pending,
    /// Transaction has been committed and included at the specified block number
    Committed(u32),
    /// Transaction has been discarded and is not included in the node
    Discarded,
}

impl fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "Pending"),
            TransactionStatus::Committed(block_number) => {
                write!(f, "Committed (Block: {})", block_number)
            },
            TransactionStatus::Discarded => write!(f, "Discarded"),
        }
    }
}

// TRANSACTION STORE UPDATE
// --------------------------------------------------------------------------------------------

/// Represents the changes that need to be applied to the client store as a result of a
/// transaction execution.
pub struct TransactionStoreUpdate {
    /// Details of the executed transaction to be inserted
    executed_transaction: ExecutedTransaction,
    /// Updated account state after the [AccountDelta] has been applied
    updated_account: Account,
    /// Information about note changes after the transaction execution.
    note_updates: NoteUpdates,
    /// New note tags to be tracked
    new_tags: Vec<NoteTagRecord>,
}

impl TransactionStoreUpdate {
    /// Creates a new [TransactionStoreUpdate] instance.
    pub fn new(
        executed_transaction: ExecutedTransaction,
        updated_account: Account,
        created_input_notes: Vec<InputNoteRecord>,
        created_output_notes: Vec<OutputNoteRecord>,
        updated_input_notes: Vec<InputNoteRecord>,
        new_tags: Vec<NoteTagRecord>,
    ) -> Self {
        Self {
            executed_transaction,
            updated_account,
            note_updates: NoteUpdates::new(
                created_input_notes,
                created_output_notes,
                updated_input_notes,
                vec![],
            ),
            new_tags,
        }
    }

    /// Returns the executed transaction.
    pub fn executed_transaction(&self) -> &ExecutedTransaction {
        &self.executed_transaction
    }

    /// Returns the updated account.
    pub fn updated_account(&self) -> &Account {
        &self.updated_account
    }

    /// Returns the note updates that need to be applied after the transaction execution.
    pub fn note_updates(&self) -> &NoteUpdates {
        &self.note_updates
    }

    /// Returns the new tags that were created as part of the transaction.
    pub fn new_tags(&self) -> &[NoteTagRecord] {
        &self.new_tags
    }
}

impl<R: FeltRng> Client<R> {
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

    /// Creates and executes a transaction specified by the request against the specified account,
    /// but does not change the local database.
    ///
    /// # Errors
    ///
    /// - Returns [ClientError::MissingOutputNotes] if the [TransactionRequest] ouput notes are not
    ///   a subset of executor's output notes
    /// - Returns a [ClientError::TransactionExecutorError] if the execution fails
    /// - Returns a [ClientError::TransactionRequestError] if the request is invalid
    #[maybe_async]
    pub fn new_transaction(
        &mut self,
        account_id: AccountId,
        transaction_request: TransactionRequest,
    ) -> Result<TransactionResult, ClientError> {
        // Validates the transaction request before executing
        maybe_await!(self.validate_request(account_id, &transaction_request))?;

        // Ensure authenticated notes have their inclusion proofs (a.k.a they're in a committed
        // state). TODO: we should consider refactoring this in a way we can handle this in
        // `get_transaction_inputs`
        let authenticated_input_note_ids: Vec<NoteId> =
            transaction_request.authenticated_input_note_ids().collect::<Vec<_>>();

        let authenticated_note_records = maybe_await!(self
            .store
            .get_input_notes(NoteFilter::List(authenticated_input_note_ids)))?;

        for authenticated_note_record in authenticated_note_records {
            if !authenticated_note_record.is_authenticated() {
                return Err(ClientError::TransactionRequestError(
                    TransactionRequestError::InputNoteNotAuthenticated,
                ));
            }
        }

        // If tx request contains unauthenticated_input_notes we should insert them
        let unauthenticated_input_notes = transaction_request
            .unauthenticated_input_notes()
            .iter()
            .cloned()
            .map(|note| note.into())
            .collect::<Vec<_>>();

        maybe_await!(self.store.upsert_input_notes(&unauthenticated_input_notes))?;

        let block_num = maybe_await!(self.store.get_sync_height())?;

        let note_ids = transaction_request.get_input_note_ids();

        let output_notes: Vec<Note> =
            transaction_request.expected_output_notes().cloned().collect();

        let future_notes: Vec<(NoteDetails, NoteTag)> =
            transaction_request.expected_future_notes().cloned().collect();

        let tx_script = transaction_request
            .build_transaction_script(maybe_await!(self.get_account_capabilities(account_id))?)?;

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
        let proven_transaction = maybe_await!(self.prove_transaction(&tx_result))?;
        self.submit_proven_transaction(proven_transaction).await?;
        maybe_await!(self.apply_transaction(tx_result))
    }

    #[maybe_async]
    fn prove_transaction(
        &mut self,
        tx_result: &TransactionResult,
    ) -> Result<ProvenTransaction, ClientError> {
        info!("Proving transaction...");

        let proven_transaction =
            maybe_await!(self.tx_prover.prove(tx_result.executed_transaction().clone().into()))?;

        info!("Transaction proven.");

        Ok(proven_transaction)
    }

    async fn submit_proven_transaction(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), ClientError> {
        info!("Submitting transaction to the network...");
        self.rpc_api.submit_proven_transaction(proven_transaction).await?;
        info!("Transaction submitted.");

        Ok(())
    }

    #[maybe_async]
    fn apply_transaction(&self, tx_result: TransactionResult) -> Result<(), ClientError> {
        let transaction_id = tx_result.executed_transaction().id();
        let sync_height = maybe_await!(self.get_sync_height())?;

        // Transaction was proven and submitted to the node correctly, persist note details and
        // update account
        info!("Applying transaction to the local store...");

        let account_id = tx_result.executed_transaction().account_id();
        let account_delta = tx_result.account_delta();
        let (mut account, _seed) = maybe_await!(self.get_account(account_id))?;

        account.apply_delta(account_delta)?;

        // Save only input notes that we care for (based on the note screener assessment)
        let created_input_notes = tx_result.relevant_notes().to_vec();
        let new_tags = created_input_notes
            .iter()
            .filter_map(|note| {
                if let InputNoteState::Expected(ExpectedNoteState { tag: Some(tag), .. }) =
                    note.state()
                {
                    Some(NoteTagRecord::with_note_source(*tag, note.id()))
                } else {
                    None
                }
            })
            .collect();

        // Save all output notes
        let created_output_notes = tx_result
            .created_notes()
            .iter()
            .cloned()
            .filter_map(|output_note| {
                OutputNoteRecord::try_from_output_note(output_note, sync_height).ok()
            })
            .collect::<Vec<_>>();

        let consumed_note_ids = tx_result.consumed_notes().iter().map(|note| note.id()).collect();
        let consumed_notes =
            maybe_await!(self.get_input_notes(NoteFilter::List(consumed_note_ids)))?;

        let mut updated_input_notes = vec![];
        for mut input_note_record in consumed_notes {
            if input_note_record.consumed_locally(account_id, transaction_id)? {
                updated_input_notes.push(input_note_record);
            }
        }

        let tx_update = TransactionStoreUpdate::new(
            tx_result.into(),
            account,
            created_input_notes,
            created_output_notes,
            updated_input_notes,
            new_tags,
        );

        maybe_await!(self.store.apply_transaction(tx_update))?;
        info!("Transaction stored.");
        Ok(())
    }

    /// Compiles the provided transaction script source and inputs into a [TransactionScript]
    pub fn compile_tx_script<T>(
        &self,
        inputs: T,
        program: &str,
    ) -> Result<TransactionScript, ClientError>
    where
        T: IntoIterator<Item = (Word, Vec<Felt>)>,
    {
        TransactionScript::compile(program, inputs, TransactionKernel::assembler())
            .map_err(ClientError::TransactionScriptError)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Helper to get the account outgoing assets.
    ///
    /// Any outgoing assets resulting from executing note scripts but not present in expected output
    /// notes would not be included.
    fn get_outgoing_assets(
        &self,
        transaction_request: &TransactionRequest,
    ) -> (BTreeMap<AccountId, u64>, BTreeSet<NonFungibleAsset>) {
        // Get own notes assets
        let mut own_notes_assets = match transaction_request.script_template() {
            Some(TransactionScriptTemplate::SendNotes(notes)) => {
                notes.iter().map(|note| (note.id(), note.assets())).collect::<BTreeMap<_, _>>()
            },
            _ => Default::default(),
        };
        // Get transaction output notes assets
        let mut output_notes_assets = transaction_request
            .expected_output_notes()
            .map(|note| (note.id(), note.assets()))
            .collect::<BTreeMap<_, _>>();

        // Merge with own notes assets and delete duplicates
        output_notes_assets.append(&mut own_notes_assets);

        // Create a map of the fungible and non-fungible assets in the output notes
        let outgoing_assets =
            output_notes_assets.values().flat_map(|note_assets| note_assets.iter());

        collect_assets(outgoing_assets)
    }

    /// Helper to get the account incoming assets.
    #[maybe_async]
    fn get_incoming_assets(
        &self,
        transaction_request: &TransactionRequest,
    ) -> Result<(BTreeMap<AccountId, u64>, BTreeSet<NonFungibleAsset>), TransactionRequestError>
    {
        // Get incoming asset notes excluding unauthenticated ones
        let incoming_notes_ids: Vec<_> = transaction_request
            .input_notes()
            .iter()
            .filter_map(|(note_id, _)| {
                if transaction_request
                    .unauthenticated_input_notes()
                    .iter()
                    .any(|note| note.id() == *note_id)
                {
                    None
                } else {
                    Some(*note_id)
                }
            })
            .collect();

        let store_input_notes =
            maybe_await!(self.get_input_notes(NoteFilter::List(incoming_notes_ids)))
                .map_err(|err| TransactionRequestError::NoteNotFound(err.to_string()))?;

        let all_incoming_assets =
            store_input_notes.iter().flat_map(|note| note.assets().iter()).chain(
                transaction_request
                    .unauthenticated_input_notes()
                    .iter()
                    .flat_map(|note| note.assets().iter()),
            );

        Ok(collect_assets(all_incoming_assets))
    }

    #[maybe_async]
    fn validate_basic_account_request(
        &self,
        transaction_request: &TransactionRequest,
        account: &Account,
    ) -> Result<(), ClientError> {
        // Get outgoing assets
        let (fungible_balance_map, non_fungible_set) =
            self.get_outgoing_assets(transaction_request);

        // Get incoming assets
        let (incoming_fungible_balance_map, incoming_non_fungible_balance_set) =
            maybe_await!(self.get_incoming_assets(transaction_request))?;

        // Check if the account balance plus incoming assets is greater than or equal to the
        // outgoing fungible assets
        for (faucet_id, amount) in fungible_balance_map {
            let account_asset_amount = account.vault().get_balance(faucet_id).unwrap_or(0);
            let incoming_balance = incoming_fungible_balance_map.get(&faucet_id).unwrap_or(&0);
            if account_asset_amount + incoming_balance < amount {
                return Err(ClientError::AssetError(AssetError::AssetAmountNotSufficient(
                    account_asset_amount,
                    amount,
                )));
            }
        }

        // Check if the account balance plus incoming assets is greater than or equal to the
        // outgoing non fungible assets
        for non_fungible in non_fungible_set {
            match account.vault().has_non_fungible_asset(non_fungible.into()) {
                Ok(true) => (),
                Ok(false) => {
                    // Check if the non fungible asset is in the incoming assets
                    if !incoming_non_fungible_balance_set.contains(&non_fungible) {
                        return Err(ClientError::AssetError(AssetError::AssetAmountNotSufficient(
                            0, 1,
                        )));
                    }
                },
                _ => {
                    return Err(ClientError::AssetError(AssetError::AssetAmountNotSufficient(
                        0, 1,
                    )));
                },
            }
        }

        Ok(())
    }

    /// Validates that the specified transaction request can be executed by the specified account.
    ///
    /// This function checks that the account has enough balance to cover the outgoing assets. This
    /// does't guarantee that the transaction will succeed, but it's useful to avoid submitting
    /// transactions that are guaranteed to fail.
    #[maybe_async]
    pub fn validate_request(
        &self,
        account_id: AccountId,
        transaction_request: &TransactionRequest,
    ) -> Result<(), ClientError> {
        let (account, _) = maybe_await!(self.get_account(account_id))?;
        if account.is_faucet() {
            // TODO(SantiagoPittella): Add faucet validations.
            Ok(())
        } else {
            maybe_await!(self.validate_basic_account_request(transaction_request, &account))
        }
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

// TESTING HELPERS
// ================================================================================================

#[cfg(feature = "testing")]
impl<R: FeltRng> Client<R> {
    #[maybe_async]
    pub fn testing_prove_transaction(
        &mut self,
        tx_result: &TransactionResult,
    ) -> Result<ProvenTransaction, ClientError> {
        maybe_await!(self.prove_transaction(tx_result))
    }

    pub async fn testing_submit_proven_transaction(
        &mut self,
        proven_transaction: ProvenTransaction,
    ) -> Result<(), ClientError> {
        self.submit_proven_transaction(proven_transaction).await
    }

    pub async fn testing_apply_transaction(
        &self,
        tx_result: TransactionResult,
    ) -> Result<(), ClientError> {
        maybe_await!(self.apply_transaction(tx_result))
    }
}

// HELPERS
// ================================================================================================

fn collect_assets<'a>(
    assets: impl Iterator<Item = &'a Asset>,
) -> (BTreeMap<AccountId, u64>, BTreeSet<NonFungibleAsset>) {
    let mut fungible_balance_map = BTreeMap::new();
    let mut non_fungible_set = BTreeSet::new();

    assets.for_each(|asset| match asset {
        Asset::Fungible(fungible) => {
            fungible_balance_map
                .entry(fungible.faucet_id())
                .and_modify(|balance| *balance += fungible.amount())
                .or_insert(fungible.amount());
        },
        Asset::NonFungible(non_fungible) => {
            non_fungible_set.insert(*non_fungible);
        },
    });

    (fungible_balance_map, non_fungible_set)
}

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

#[cfg(test)]
mod test {
    use miden_lib::{accounts::auth::RpoFalcon512, transaction::TransactionKernel};
    use miden_objects::{
        accounts::{
            account_id::testing::{
                ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
                ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            },
            AccountBuilder, AccountComponent, AccountData, StorageMap, StorageSlot,
        },
        assets::{Asset, FungibleAsset},
        crypto::dsa::rpo_falcon512::SecretKey,
        notes::NoteType,
        testing::account_component::BASIC_WALLET_CODE,
        Felt, FieldElement, Word,
    };

    use super::{PaymentTransactionData, TransactionRequest};
    use crate::mock::create_test_client;

    #[tokio::test]
    async fn test_transaction_creates_two_notes() {
        let (mut client, _) = create_test_client();
        let asset_1: Asset =
            FungibleAsset::new(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN.try_into().unwrap(), 123)
                .unwrap()
                .into();
        let asset_2: Asset =
            FungibleAsset::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap(), 500)
                .unwrap()
                .into();

        let secret_key = SecretKey::new();

        let wallet_component = AccountComponent::compile(
            BASIC_WALLET_CODE,
            TransactionKernel::assembler(),
            vec![StorageSlot::Value(Word::default()), StorageSlot::Map(StorageMap::default())],
        )
        .unwrap()
        .with_supports_all_types();

        let (account, _) = AccountBuilder::new()
            .init_seed(Default::default())
            .nonce(Felt::ONE)
            .with_component(wallet_component)
            .with_component(RpoFalcon512::new(secret_key.public_key()))
            .with_assets([asset_1, asset_2])
            .build_testing()
            .unwrap();

        client
            .import_account(AccountData::new(
                account.clone(),
                None,
                miden_objects::accounts::AuthSecretKey::RpoFalcon512(secret_key.clone()),
            ))
            .unwrap();
        client.sync_state().await.unwrap();
        let tx_request = TransactionRequest::pay_to_id(
            PaymentTransactionData::new(
                vec![asset_1, asset_2],
                account.id(),
                ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
            ),
            None,
            NoteType::Private,
            client.rng(),
        )
        .unwrap();

        let tx_result = client.new_transaction(account.id(), tx_request).unwrap();
        assert!(tx_result
            .created_notes()
            .get_note(0)
            .assets()
            .is_some_and(|assets| assets.num_assets() == 2));
        // Prove and apply transaction
        client.testing_apply_transaction(tx_result.clone()).await.unwrap();
    }
}
