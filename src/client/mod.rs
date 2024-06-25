use alloc::rc::Rc;

use miden_objects::{
    accounts::AccountId,
    crypto::rand::FeltRng,
    notes::{NoteExecutionHint, NoteTag, NoteType},
    NoteError,
};
use miden_tx::{auth::TransactionAuthenticator, TransactionExecutor};
use tracing::info;
use winter_maybe_async::{maybe_async, maybe_await};

use crate::{
    errors::IdPrefixFetchError,
    store::{data_store::ClientDataStore, InputNoteRecord, NoteFilter as ClientNoteFilter, Store},
};

pub mod rpc;
use rpc::NodeRpcClient;

pub mod accounts;
mod note_screener;
mod notes;
pub mod store_authenticator;
pub mod sync;
pub mod transactions;
pub use note_screener::{NoteConsumability, NoteRelevance, NoteScreener};

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
pub struct Client<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> {
    /// The client's store, which provides a way to write and read entities to provide persistence.
    store: Rc<S>,
    /// An instance of [FeltRng] which provides randomness tools for generating new keys,
    /// serial numbers, etc.
    rng: R,
    /// An instance of [NodeRpcClient] which provides a way for the client to connect to the
    /// Miden node.
    rpc_api: N,
    tx_executor: TransactionExecutor<ClientDataStore<S>, A>,
}

impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> Client<N, R, S, A> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client].
    ///
    /// ## Arguments
    ///
    /// - `api`: An instance of [NodeRpcClient] which provides a way for the client to connect
    ///   to the Miden node.
    /// - `store`: An instance of [Store], which provides a way to write and read entities to
    ///   provide persistence.
    /// - `executor_store`: An instance of [Store] that provides a way for [TransactionExecutor] to
    ///   retrieve relevant inputs at the moment of transaction execution. It should be the same
    ///   store as the one for `store`, but it doesn't have to be the **same instance**.
    /// - `authenticator`: Defines the transaction authenticator that will be used by the
    ///   transaction executor whenever a signature is requested from within the VM.
    /// - `in_debug_mode`: Instantiates the transaction executor (and in turn, its compiler)
    ///   in debug mode, which will enable debug logs for scripts compiled with this mode for
    ///   easier MASM debugging.
    ///
    /// # Errors
    ///
    /// Returns an error if the client could not be instantiated.
    pub fn new(api: N, rng: R, store: Rc<S>, authenticator: A, in_debug_mode: bool) -> Self {
        if in_debug_mode {
            info!("Creating the Client in debug mode.");
        }

        let data_store = ClientDataStore::new(store.clone());
        let authenticator = Some(Rc::new(authenticator));
        let tx_executor = TransactionExecutor::new(data_store, authenticator);

        Self { store, rng, rpc_api: api, tx_executor }
    }

    // TEST HELPERS
    // --------------------------------------------------------------------------------------------

    #[cfg(any(test, feature = "testing"))]
    pub fn rpc_api(&mut self) -> &mut N {
        &mut self.rpc_api
    }

    // The miden-wasm crate needs access to the store for now, so we expose it here.
    #[cfg(any(test, feature = "async"))]
    pub fn store(&mut self) -> &S {
        &self.store
    }

    #[cfg(any(test, feature = "testing"))]
    #[winter_maybe_async::maybe_async]
    pub fn get_block_headers(
        &self,
        block_numbers: &[u32],
    ) -> Result<Vec<(miden_objects::BlockHeader, bool)>, crate::errors::ClientError> {
        let result = winter_maybe_async::maybe_await!(self.store.get_block_headers(block_numbers))?;
        Ok(result)
    }
}

// This is duplicated in src/cli/mod.rs. We should move it to a common place because the miden-wasm
// crate also needs access to it.
#[maybe_async]
pub fn get_input_note_with_id_prefix<
    N: NodeRpcClient,
    R: FeltRng,
    S: Store,
    A: TransactionAuthenticator,
>(
    client: &Client<N, R, S, A>,
    note_id_prefix: &str,
) -> Result<InputNoteRecord, IdPrefixFetchError> {
    let mut input_note_records = maybe_await!(client.get_input_notes(ClientNoteFilter::All))
        .map_err(|err| {
            tracing::error!("Error when fetching all notes from the store: {err}");
            IdPrefixFetchError::NoMatch(format!("note ID prefix {note_id_prefix}").to_string())
        })?
        .into_iter()
        .filter(|note_record| note_record.id().to_hex().starts_with(note_id_prefix))
        .collect::<Vec<_>>();

    if input_note_records.is_empty() {
        return Err(IdPrefixFetchError::NoMatch(
            format!("note ID prefix {note_id_prefix}").to_string(),
        ));
    }
    if input_note_records.len() > 1 {
        let input_note_record_ids = input_note_records
            .iter()
            .map(|input_note_record| input_note_record.id())
            .collect::<Vec<_>>();
        tracing::error!(
            "Multiple notes found for the prefix {}: {:?}",
            note_id_prefix,
            input_note_record_ids
        );
        return Err(IdPrefixFetchError::MultipleMatches(
            format!("note ID prefix {note_id_prefix}").to_string(),
        ));
    }

    Ok(input_note_records
        .pop()
        .expect("input_note_records should always have one element"))
}

// This is duplicated in src/cli/mod.rs. We should move it to a common place because the miden-wasm
// crate also needs access to it.
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

    let execution = NoteExecutionHint::Local;
    match note_type {
        NoteType::Public => NoteTag::for_public_use_case(SWAP_USE_CASE_ID, payload, execution),
        _ => NoteTag::for_local_use_case(SWAP_USE_CASE_ID, payload),
    }
}
