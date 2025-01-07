use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{future::Future, pin::Pin};

use miden_objects::{
    accounts::{Account, AccountHeader, AccountId},
    crypto::merkle::MmrDelta,
    notes::{NoteId, NoteTag, Nullifier},
    BlockHeader, Digest,
};

use super::get_nullifier_prefix;
use crate::{
    rpc::{
        domain::{
            notes::CommittedNote, nullifiers::NullifierUpdate, transactions::TransactionUpdate,
        },
        NodeRpcClient,
    },
    store::InputNoteRecord,
    ClientError,
};

pub struct RelevantSyncInfo {
    pub block_header: BlockHeader,
    pub expected_note_inclusions: Vec<CommittedNote>,
    pub new_notes: Vec<InputNoteRecord>,
    pub nullifiers: Vec<NullifierUpdate>,
    pub committed_transactions: Vec<TransactionUpdate>,
    pub updated_public_accounts: Vec<Account>,
    pub mismatched_private_accounts: Vec<(AccountId, Digest)>,
    pub mmr_delta: MmrDelta,
}

/// Gives information about the status of the sync process after a step.
pub enum SyncStatus {
    SyncedToLastBlock(RelevantSyncInfo),
    SyncedToBlock(RelevantSyncInfo),
}

impl SyncStatus {
    pub fn into_relevant_sync_info(self) -> RelevantSyncInfo {
        match self {
            SyncStatus::SyncedToLastBlock(info) => info,
            SyncStatus::SyncedToBlock(info) => info,
        }
    }
}

type NewNoteFilter = Box<
    dyn Fn(CommittedNote, BlockHeader) -> Pin<Box<dyn Future<Output = Option<InputNoteRecord>>>>,
>;

/// The state sync components encompasses the client's sync logic.
///
/// When created it receives the current state of the client's relevant elements (block, accounts,
/// notes, etc). It is then used to requset updates from the node and apply them to the relevant
/// elements. The updates are then returned and can be applied to the store to persist the changes.
pub struct StateSync {
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    account_states: Vec<AccountHeader>,
    current_block: BlockHeader,
    note_tags: Vec<NoteTag>,
    expected_note_ids: Vec<NoteId>,
    new_note_filter: NewNoteFilter,
    unspent_nullifiers: Vec<Nullifier>,
}

impl StateSync {
    /// Creates a new instance of the state sync component.
    ///
    /// # Arguments
    ///
    /// * `rpc_api` - The RPC client to use to communicate with the node.
    /// * `current_block` - The latest block header tracked by the client.
    /// * `current_block_has_relevant_notes` - A flag indicating if the current block has notes that
    ///   are relevant to the client.
    /// * `accounts` - The headers of accounts tracked by the client.
    /// * `note_tags` - The note tags to be used in the sync state request.
    /// * `unspent_input_notes` - The input notes that haven't been yet consumed and may be changed
    ///   in the sync process.
    /// * `unspent_output_notes` - The output notes that haven't been yet consumed and may be
    ///   changed in the sync process.
    /// * `current_partial_mmr` - The current partial MMR of the client.
    pub fn new(
        rpc_api: Arc<dyn NodeRpcClient + Send>,
        current_block: BlockHeader,
        account_states: Vec<AccountHeader>,
        note_tags: Vec<NoteTag>,
        expected_note_ids: Vec<NoteId>,
        new_note_filter: NewNoteFilter,
        unspent_nullifiers: Vec<Nullifier>,
    ) -> Self {
        Self {
            rpc_api,
            current_block,
            note_tags,
            expected_note_ids,
            new_note_filter,
            account_states,
            unspent_nullifiers,
        }
    }

    /// Executes a single step of the state sync process, returning the changes that should be
    /// applied to the store.
    ///
    /// A step in this context means a single request to the node to get the next relevant block and
    /// the changes that happened in it. This block may not be the last one in the chain and
    /// the client may need to call this method multiple times until it reaches the chain tip.
    /// Wheter or not the client has reached the chain tip is indicated by the returned
    /// [SyncStatus] variant. `None` is returned if the client is already synced with the chain tip
    /// and there are no new changes.
    pub async fn sync_state_step(self) -> Result<Option<SyncStatus>, ClientError> {
        let current_block_num = self.current_block.block_num();
        let account_ids: Vec<AccountId> = self.account_states.iter().map(|acc| acc.id()).collect();

        // To receive information about added nullifiers, we reduce them to the higher 16 bits
        // Note that besides filtering by nullifier prefixes, the node also filters by block number
        // (it only returns nullifiers from current_block_num until
        // response.block_header.block_num())
        let nullifiers_tags: Vec<u16> =
            self.unspent_nullifiers.iter().map(get_nullifier_prefix).collect();

        let response = self
            .rpc_api
            .sync_state(current_block_num, &account_ids, &self.note_tags, &nullifiers_tags)
            .await?;

        // We don't need to continue if the chain has not advanced, there are no new changes
        if response.block_header.block_num() == current_block_num {
            return Ok(None);
        }

        let mut expected_note_inclusions = vec![];
        let mut relevant_new_notes = vec![];

        for committed_note in response.note_inclusions {
            if self.expected_note_ids.contains(committed_note.note_id()) {
                expected_note_inclusions.push(committed_note.clone());
            } else if let Some(new_note) =
                (self.new_note_filter)(committed_note, response.block_header).await
            {
                relevant_new_notes.push(new_note);
            }
        }

        let (updated_public_accounts, mismatched_private_accounts) =
            self.account_state_sync(&response.account_hash_updates).await?;

        let relevant_nullifiers = response
            .nullifiers
            .into_iter()
            .filter(|nullifier_update| {
                self.unspent_nullifiers.contains(&nullifier_update.nullifier)
            })
            .collect();

        let info = RelevantSyncInfo {
            block_header: response.block_header,
            expected_note_inclusions,
            new_notes: relevant_new_notes,
            nullifiers: relevant_nullifiers,
            committed_transactions: response.transactions,
            updated_public_accounts,
            mismatched_private_accounts,
            mmr_delta: response.mmr_delta,
        };

        if response.chain_tip == response.block_header.block_num() {
            Ok(Some(SyncStatus::SyncedToLastBlock(info)))
        } else {
            Ok(Some(SyncStatus::SyncedToBlock(info)))
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Compares the state of tracked accounts with the updates received from the node and returns
    /// the accounts that need to be updated.
    ///
    /// When a mismatch is detected, two scenarios are possible:
    /// * If the account is public, the component will request the node for the updated account
    ///   details.
    /// * If the account is private it will be marked as mismatched and the client will need to
    ///   handle it (it could be a stale account state or a reason to lock the account).
    async fn account_state_sync(
        &self,
        account_hash_updates: &[(AccountId, Digest)],
    ) -> Result<(Vec<Account>, Vec<(AccountId, Digest)>), ClientError> {
        let (public_accounts, private_accounts): (Vec<_>, Vec<_>) =
            self.account_states.iter().partition(|acc| acc.id().is_public());

        let updated_public_accounts =
            self.get_updated_public_accounts(account_hash_updates, &public_accounts).await?;

        let mismatched_private_accounts = account_hash_updates
            .iter()
            .filter(|(new_id, new_hash)| {
                private_accounts
                    .iter()
                    .any(|acc| acc.id() == *new_id && acc.hash() != *new_hash)
            })
            .cloned()
            .collect::<Vec<_>>();

        Ok((updated_public_accounts, mismatched_private_accounts))
    }

    async fn get_updated_public_accounts(
        &self,
        account_updates: &[(AccountId, Digest)],
        current_public_accounts: &[&AccountHeader],
    ) -> Result<Vec<Account>, ClientError> {
        let mut mismatched_public_accounts = vec![];

        for (id, hash) in account_updates {
            // check if this updated account is tracked by the client
            if let Some(account) = current_public_accounts
                .iter()
                .find(|acc| *id == acc.id() && *hash != acc.hash())
            {
                mismatched_public_accounts.push(*account);
            }
        }

        self.rpc_api
            .get_updated_public_accounts(&mismatched_public_accounts)
            .await
            .map_err(ClientError::RpcError)
    }
}

/// Queries the node for the received note that isn't being locally tracked in the client.
///
/// The client can receive metadata for private notes that it's not tracking. In this case,
/// notes are ignored for now as they become useless until details are imported.
pub(crate) async fn fetch_public_note_details(
    rpc_api: Arc<dyn NodeRpcClient + Send>,
    note_id: NoteId,
    block_header: BlockHeader,
) -> Option<InputNoteRecord> {
    let mut return_notes = rpc_api.get_public_note_records(&[note_id], None).await.ok()?;

    if let Some(mut note) = return_notes.pop() {
        note.block_header_received(block_header).ok()?;
        Some(note)
    } else {
        None
    }
}
