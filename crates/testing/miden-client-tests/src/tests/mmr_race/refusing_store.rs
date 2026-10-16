//! A store that refuses chain writes, counts calls and holds calls when a test asks it to.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use miden_client::Word;
use miden_client::account::{
    Account,
    AccountCode,
    AccountHeader,
    AccountId,
    AccountStorage,
    Address,
    StorageMapKey,
    StorageSlotName,
};
use miden_client::asset::{Asset, AssetVault, AssetWitness};
use miden_client::block::{AccountWitness, BlockHeader};
use miden_client::crypto::{InOrderIndex, MmrPeaks, PartialMmr};
use miden_client::note::{BlockNumber, NoteScript, NoteTag, Nullifier};
use miden_client::store::{
    AccountRecord,
    AccountStatus,
    AccountStorageFilter,
    BlockRelevance,
    ClientAccountType,
    InputNoteCursor,
    InputNoteRecord,
    NoteFilter,
    OutputNoteRecord,
    PartialBlockchainFilter,
    SettingMutation,
    SettingScope,
    Store,
    StoreError,
    TransactionFilter,
};
use miden_client::sync::{NoteTagRecord, StateSyncUpdate};
use miden_client::transaction::{TransactionRecord, TransactionStoreUpdate};
use miden_client_sqlite_store::SqliteStore;
use miden_protocol::Felt;
use miden_protocol::account::StorageMapWitness;
use miden_protocol::asset::AssetId;

use super::Gate;

/// Counts the calls of one store method, and refuses the next calls that a test asks for.
#[derive(Default)]
pub(super) struct Refusals {
    refused: AtomicUsize,
    calls: AtomicUsize,
}

impl Refusals {
    /// Refuses the next `count` calls.
    pub(super) fn refuse_next(&self, count: usize) {
        self.refused.store(count, Ordering::SeqCst);
    }

    /// Returns the number of calls, including the refused ones.
    pub(super) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// Counts a call, and returns the error of a stale chain write if the call is refused.
    fn check(&self) -> Result<(), StoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self
            .refused
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| count.checked_sub(1))
            .is_ok()
        {
            return Err(StoreError::DatabaseTransientError(
                "stale chain write: the test store refused the write".to_string(),
            ));
        }
        Ok(())
    }
}

/// Delegates to a [`SqliteStore`]. A refused call writes nothing and returns the error that the
/// `SQLite` store returns for a stale chain write.
pub(super) struct RefusingStore {
    inner: SqliteStore,
    pub(super) syncs: Refusals,
    pub(super) prunes: Refusals,
    pub(super) inserts: Refusals,
    /// The number of `get_current_partial_mmr` calls.
    partial_mmr_reads: AtomicUsize,
    /// Holds the `insert_block_header` call of the block that [`Self::hold_insert_of`] names,
    /// before the call is refused or reaches `SQLite`.
    pub(super) insert_gate: Gate,
    held_insert: Mutex<Option<BlockNumber>>,
    /// Holds a `get_current_partial_mmr` call after `SQLite` answers it.
    pub(super) read_gate: Gate,
    /// Holds a `get_current_blockchain_peaks` call after `SQLite` answers it.
    pub(super) peaks_gate: Gate,
}

impl RefusingStore {
    pub(super) async fn new(store_path: &Path) -> Self {
        Self {
            inner: SqliteStore::new(store_path.to_path_buf()).await.unwrap(),
            syncs: Refusals::default(),
            prunes: Refusals::default(),
            inserts: Refusals::default(),
            partial_mmr_reads: AtomicUsize::new(0),
            insert_gate: Gate::default(),
            held_insert: Mutex::new(None),
            read_gate: Gate::default(),
            peaks_gate: Gate::default(),
        }
    }

    /// Arms [`Self::insert_gate`] for the next insert of the header of `block`.
    pub(super) fn hold_insert_of(&self, block: BlockNumber) {
        *self.held_insert.lock().unwrap() = Some(block);
        self.insert_gate.arm();
    }

    pub(super) fn partial_mmr_reads(&self) -> usize {
        self.partial_mmr_reads.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl Store for RefusingStore {
    fn identifier(&self) -> &str {
        self.inner.identifier()
    }

    fn get_current_timestamp(&self) -> Option<u64> {
        self.inner.get_current_timestamp()
    }

    async fn get_note_tags(&self) -> Result<Vec<NoteTagRecord>, StoreError> {
        self.inner.get_note_tags().await
    }

    async fn get_unique_note_tags(&self) -> Result<BTreeSet<NoteTag>, StoreError> {
        self.inner.get_unique_note_tags().await
    }

    async fn add_note_tag(&self, tag: NoteTagRecord) -> Result<bool, StoreError> {
        self.inner.add_note_tag(tag).await
    }

    async fn remove_note_tag(&self, tag: NoteTagRecord) -> Result<usize, StoreError> {
        self.inner.remove_note_tag(tag).await
    }

    async fn get_sync_height(&self) -> Result<BlockNumber, StoreError> {
        self.inner.get_sync_height().await
    }

    async fn apply_state_sync(&self, state_sync_update: StateSyncUpdate) -> Result<(), StoreError> {
        self.syncs.check()?;
        self.inner.apply_state_sync(state_sync_update).await
    }

    async fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<TransactionRecord>, StoreError> {
        self.inner.get_transactions(filter).await
    }

    async fn apply_transaction(&self, tx_update: TransactionStoreUpdate) -> Result<(), StoreError> {
        self.inner.apply_transaction(tx_update).await
    }

    async fn apply_transaction_batch(
        &self,
        tx_updates: Vec<TransactionStoreUpdate>,
    ) -> Result<(), StoreError> {
        self.inner.apply_transaction_batch(tx_updates).await
    }

    async fn get_input_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<InputNoteRecord>, StoreError> {
        self.inner.get_input_notes(filter).await
    }

    async fn get_output_notes(
        &self,
        filter: NoteFilter,
    ) -> Result<Vec<OutputNoteRecord>, StoreError> {
        self.inner.get_output_notes(filter).await
    }

    async fn get_input_note_after(
        &self,
        filter: NoteFilter,
        consumer: AccountId,
        block_start: Option<BlockNumber>,
        block_end: Option<BlockNumber>,
        cursor: Option<InputNoteCursor>,
    ) -> Result<Option<InputNoteRecord>, StoreError> {
        self.inner
            .get_input_note_after(filter, consumer, block_start, block_end, cursor)
            .await
    }

    async fn upsert_input_notes(&self, notes: &[InputNoteRecord]) -> Result<(), StoreError> {
        self.inner.upsert_input_notes(notes).await
    }

    async fn get_note_script(&self, script_root: Word) -> Result<NoteScript, StoreError> {
        self.inner.get_note_script(script_root).await
    }

    async fn upsert_note_scripts(&self, note_scripts: &[NoteScript]) -> Result<(), StoreError> {
        self.inner.upsert_note_scripts(note_scripts).await
    }

    async fn insert_block_header(
        &self,
        block_header: &BlockHeader,
        nodes: &[(InOrderIndex, Word)],
        has_client_notes: bool,
    ) -> Result<(), StoreError> {
        let held = *self.held_insert.lock().unwrap();
        if held == Some(block_header.block_num()) {
            self.insert_gate.pass().await;
        }
        self.inserts.check()?;
        self.inner.insert_block_header(block_header, nodes, has_client_notes).await
    }

    async fn untrack_and_prune_irrelevant_blocks(
        &self,
        blocks_to_untrack: &[BlockNumber],
        node_indices_to_remove: &[InOrderIndex],
    ) -> Result<(), StoreError> {
        self.prunes.check()?;
        self.inner
            .untrack_and_prune_irrelevant_blocks(blocks_to_untrack, node_indices_to_remove)
            .await
    }

    async fn prune_account_history(
        &self,
        account_id: AccountId,
        up_to_nonce: Felt,
    ) -> Result<usize, StoreError> {
        self.inner.prune_account_history(account_id, up_to_nonce).await
    }

    async fn get_block_headers(
        &self,
        block_numbers: &BTreeSet<BlockNumber>,
    ) -> Result<Vec<(BlockHeader, BlockRelevance)>, StoreError> {
        self.inner.get_block_headers(block_numbers).await
    }

    async fn get_tracked_block_headers(&self) -> Result<Vec<BlockHeader>, StoreError> {
        self.inner.get_tracked_block_headers().await
    }

    async fn get_tracked_block_header_numbers(&self) -> Result<BTreeSet<usize>, StoreError> {
        self.inner.get_tracked_block_header_numbers().await
    }

    async fn get_partial_blockchain_nodes(
        &self,
        filter: PartialBlockchainFilter,
    ) -> Result<BTreeMap<InOrderIndex, Word>, StoreError> {
        self.inner.get_partial_blockchain_nodes(filter).await
    }

    async fn get_current_blockchain_peaks(&self) -> Result<MmrPeaks, StoreError> {
        let peaks = self.inner.get_current_blockchain_peaks().await;
        self.peaks_gate.pass().await;
        peaks
    }

    async fn get_current_partial_mmr(&self) -> Result<PartialMmr, StoreError> {
        self.partial_mmr_reads.fetch_add(1, Ordering::SeqCst);
        let partial_mmr = self.inner.get_current_partial_mmr().await;
        self.read_gate.pass().await;
        partial_mmr
    }

    async fn insert_account(
        &self,
        account: &Account,
        initial_address: Address,
        client_account_type: ClientAccountType,
    ) -> Result<(), StoreError> {
        self.inner.insert_account(account, initial_address, client_account_type).await
    }

    async fn update_account(&self, account: &Account) -> Result<(), StoreError> {
        self.inner.update_account(account).await
    }

    async fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        self.inner.get_account_ids().await
    }

    async fn get_account_headers(&self) -> Result<Vec<(AccountHeader, AccountStatus)>, StoreError> {
        self.inner.get_account_headers().await
    }

    async fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<Option<(AccountHeader, AccountStatus)>, StoreError> {
        self.inner.get_account_header(account_id).await
    }

    async fn get_account_header_by_commitment(
        &self,
        account_commitment: Word,
    ) -> Result<Option<AccountHeader>, StoreError> {
        self.inner.get_account_header_by_commitment(account_commitment).await
    }

    async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountRecord>, StoreError> {
        self.inner.get_account(account_id).await
    }

    async fn get_account_code(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountCode>, StoreError> {
        self.inner.get_account_code(account_id).await
    }

    async fn upsert_foreign_account_code(
        &self,
        account_id: AccountId,
        code: AccountCode,
    ) -> Result<(), StoreError> {
        self.inner.upsert_foreign_account_code(account_id, code).await
    }

    async fn get_foreign_account_code(
        &self,
        account_ids: Vec<AccountId>,
    ) -> Result<BTreeMap<AccountId, AccountCode>, StoreError> {
        self.inner.get_foreign_account_code(account_ids).await
    }

    async fn track_account_witness(&self, account_id: AccountId) -> Result<bool, StoreError> {
        self.inner.track_account_witness(account_id).await
    }

    async fn untrack_account_witness(&self, account_id: AccountId) -> Result<bool, StoreError> {
        self.inner.untrack_account_witness(account_id).await
    }

    async fn tracked_account_witnesses(&self) -> Result<Vec<AccountId>, StoreError> {
        self.inner.tracked_account_witnesses().await
    }

    async fn get_account_witness(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountWitness>, StoreError> {
        self.inner.get_account_witness(account_id).await
    }

    async fn update_account_witness(
        &self,
        account_id: AccountId,
        witness: &AccountWitness,
    ) -> Result<bool, StoreError> {
        self.inner.update_account_witness(account_id, witness).await
    }

    async fn set_setting(
        &self,
        scope: SettingScope,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), StoreError> {
        self.inner.set_setting(scope, key, value).await
    }

    async fn get_setting(
        &self,
        scope: SettingScope,
        key: String,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner.get_setting(scope, key).await
    }

    async fn remove_setting(&self, scope: SettingScope, key: String) -> Result<bool, StoreError> {
        self.inner.remove_setting(scope, key).await
    }

    async fn list_setting_keys(&self, scope: SettingScope) -> Result<Vec<String>, StoreError> {
        self.inner.list_setting_keys(scope).await
    }

    async fn apply_settings_mutations(
        &self,
        scope: SettingScope,
        mutations: Vec<SettingMutation>,
    ) -> Result<(), StoreError> {
        self.inner.apply_settings_mutations(scope, mutations).await
    }

    async fn get_unspent_input_note_nullifiers(&self) -> Result<Vec<Nullifier>, StoreError> {
        self.inner.get_unspent_input_note_nullifiers().await
    }

    async fn get_account_vault(&self, account_id: AccountId) -> Result<AssetVault, StoreError> {
        self.inner.get_account_vault(account_id).await
    }

    async fn get_account_assets(&self, account_id: AccountId) -> Result<Vec<Asset>, StoreError> {
        self.inner.get_account_assets(account_id).await
    }

    async fn get_vault_asset_witnesses(
        &self,
        account_id: AccountId,
        vault_root: Word,
        asset_ids: BTreeSet<AssetId>,
    ) -> Result<Vec<AssetWitness>, StoreError> {
        self.inner.get_vault_asset_witnesses(account_id, vault_root, asset_ids).await
    }

    async fn get_account_asset(
        &self,
        account_id: AccountId,
        asset_id: AssetId,
    ) -> Result<Option<(Asset, AssetWitness)>, StoreError> {
        self.inner.get_account_asset(account_id, asset_id).await
    }

    async fn get_account_map_item(
        &self,
        account_id: AccountId,
        slot_name: StorageSlotName,
        key: StorageMapKey,
    ) -> Result<(Word, StorageMapWitness), StoreError> {
        self.inner.get_account_map_item(account_id, slot_name, key).await
    }

    async fn get_account_storage(
        &self,
        account_id: AccountId,
        filter: AccountStorageFilter,
    ) -> Result<AccountStorage, StoreError> {
        self.inner.get_account_storage(account_id, filter).await
    }

    async fn get_addresses_by_account_id(
        &self,
        account_id: AccountId,
    ) -> Result<Vec<Address>, StoreError> {
        self.inner.get_addresses_by_account_id(account_id).await
    }

    async fn insert_address(
        &self,
        address: Address,
        account_id: AccountId,
    ) -> Result<(), StoreError> {
        self.inner.insert_address(address, account_id).await
    }

    async fn remove_address(&self, address: Address) -> Result<bool, StoreError> {
        self.inner.remove_address(address).await
    }

    async fn get_minimal_partial_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountRecord>, StoreError> {
        self.inner.get_minimal_partial_account(account_id).await
    }
}
