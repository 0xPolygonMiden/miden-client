//! Two client instances on one store, and a store that already lacks part of a tracked path.
//!
//! The race tests hold one client at a point between its read of the partial MMR and its write, let
//! the other client change the chain data, then let the first client continue. The heal tests
//! remove stored nodes with raw SQL, as an older client could leave them.

use std::collections::{BTreeMap, BTreeSet};
use std::env::temp_dir;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use miden_client::builder::ClientBuilder;
use miden_client::crypto::{Forest, InOrderIndex, PartialMmr};
use miden_client::keystore::FilesystemKeyStore;
use miden_client::note::{BlockNumber, NoteFile, NoteSyncHint};
use miden_client::rpc::domain::account::{AccountProof, GetAccountRequest};
use miden_client::rpc::domain::account_vault::AccountVaultInfo;
use miden_client::rpc::domain::note::{FetchedNote, SyncNotesBlock, SyncedNote};
use miden_client::rpc::domain::nullifier::NullifierUpdate;
use miden_client::rpc::domain::storage_map::StorageMapInfo;
use miden_client::rpc::domain::sync::{ChainMmrInfo, SyncTarget};
use miden_client::rpc::domain::transaction::TransactionRecord;
use miden_client::rpc::encryption::{AttestedTransactionEncryptionKey, SealedTransactionInputs};
use miden_client::rpc::{NetworkNoteStatusInfo, NodeRpcClient, RpcError, RpcLimits, RpcStatusInfo};
use miden_client::store::{PartialBlockchainFilter, Store, StoreError};
use miden_client::sync::{NoteObserver, StateSync, StateSyncUpdate};
use miden_client::testing::common::create_test_store_path;
use miden_client::testing::mock::{MockClient, MockRpcApi};
use miden_client::utils::Serializable;
use miden_client::{ClientError, Word};
use miden_client_sqlite_store::{ClientBuilderSqliteExt, SqliteStore};
use miden_protocol::Felt;
use miden_protocol::account::AccountId;
use miden_protocol::address::NetworkId;
use miden_protocol::batch::{ProposedBatch, ProvenBatch};
use miden_protocol::block::{BlockHeader, SignedBlock};
use miden_protocol::crypto::merkle::mmr::{MmrError, MmrProof};
use miden_protocol::note::{Note, NoteId, NoteScript, NoteTag, NoteType};
use miden_protocol::transaction::{ProvenTransaction, RawOutputNote};
use miden_protocol::vm::ExecutionProof;
use miden_standards::testing::note::NoteBuilder;
use miden_testing::MockChainBuilder;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::tests::seed_mock_transaction_encryption_key;

type Client = MockClient<FilesystemKeyStore>;

/// The block that commits the note in the single-note tests.
const NOTE_BLOCK: u32 = 1;

/// The sync height of the heal tests. At forest 71, leaf 1 is in the tree of 64 leaves and its path
/// has 6 siblings.
const HEAL_HEIGHT: u32 = 70;

// GATES
// ================================================================================================

/// Holds the first call that reaches it after [`Gate::arm`], until [`while_held`] releases it.
#[derive(Default)]
struct Gate {
    armed: AtomicBool,
    started: AtomicBool,
    released: AtomicBool,
}

impl Gate {
    fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    async fn pass(&self) {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.started.store(true, Ordering::SeqCst);
            while !self.released.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        }
    }
}

/// Runs `held` until it reaches `gate`, then runs `during`, then lets `held` continue.
///
/// If `held` ends without reaching the gate, `during` does not run and the second value is `None`.
async fn while_held<T, U>(
    gate: &Gate,
    held: impl Future<Output = T>,
    during: impl Future<Output = U>,
) -> (T, Option<U>) {
    let held_done = AtomicBool::new(false);
    tokio::join!(
        async {
            let output = held.await;
            held_done.store(true, Ordering::SeqCst);
            output
        },
        async {
            while !gate.started.load(Ordering::SeqCst) {
                if held_done.load(Ordering::SeqCst) {
                    return None;
                }
                tokio::task::yield_now().await;
            }
            let output = during.await;
            gate.released.store(true, Ordering::SeqCst);
            Some(output)
        }
    )
}

/// Holds the sync that it is attached to in the hook that runs before the sync writes.
struct GateObserver(Arc<Gate>);

#[async_trait::async_trait(?Send)]
impl NoteObserver for GateObserver {
    fn name(&self) -> &'static str {
        "gate"
    }

    async fn observe(&self, _note: &SyncedNote) -> Result<bool, ClientError> {
        Ok(false)
    }

    async fn apply(&self, _sync_update: &StateSyncUpdate) -> Result<(), ClientError> {
        self.0.pass().await;
        Ok(())
    }
}

// GATED RPC
// ================================================================================================

/// Delegates to the mock node, with switches for the block proof requests and the note sync.
///
/// `sync_notes` returns each path at the forest of `block_to`, as the node does. The mock node
/// returns the paths at its current tip instead.
struct GatedRpc {
    inner: Arc<MockRpcApi>,
    /// Holds a block header request that includes an MMR proof.
    proof_gate: Gate,
    /// Holds a note sync request.
    sync_notes_gate: Gate,
    /// Makes each block proof request fail as if the node were not reachable.
    fail_proofs: AtomicBool,
    /// Returns each block proof at this forest, as a node that is behind the client does.
    proof_forest: Mutex<Option<Forest>>,
    /// The number of block proof requests, including the failed ones.
    proof_requests: AtomicUsize,
}

impl GatedRpc {
    fn new(inner: Arc<MockRpcApi>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            proof_gate: Gate::default(),
            sync_notes_gate: Gate::default(),
            fail_proofs: AtomicBool::new(false),
            proof_forest: Mutex::new(None),
            proof_requests: AtomicUsize::new(0),
        })
    }

    fn proof_requests(&self) -> usize {
        self.proof_requests.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl NodeRpcClient for GatedRpc {
    async fn set_genesis_commitment(&self, commitment: Word) -> Result<(), RpcError> {
        self.inner.set_genesis_commitment(commitment).await
    }

    fn has_genesis_commitment(&self) -> Option<Word> {
        self.inner.has_genesis_commitment()
    }

    async fn get_transaction_encryption_key(
        &self,
    ) -> Result<AttestedTransactionEncryptionKey, RpcError> {
        self.inner.get_transaction_encryption_key().await
    }

    async fn submit_proven_transaction(
        &self,
        proven_transaction: &ProvenTransaction,
        sealed_transaction_inputs: SealedTransactionInputs,
    ) -> Result<BlockNumber, RpcError> {
        self.inner
            .submit_proven_transaction(proven_transaction, sealed_transaction_inputs)
            .await
    }

    async fn submit_proven_batch(
        &self,
        proven_batch: &ProvenBatch,
        proposed_batch: &ProposedBatch,
        sealed_transaction_inputs: Vec<SealedTransactionInputs>,
    ) -> Result<BlockNumber, RpcError> {
        self.inner
            .submit_proven_batch(proven_batch, proposed_batch, sealed_transaction_inputs)
            .await
    }

    async fn get_block_header_by_number(
        &self,
        block_num: Option<BlockNumber>,
        include_mmr_proof: bool,
    ) -> Result<(BlockHeader, Option<MmrProof>), RpcError> {
        if !include_mmr_proof {
            return self.inner.get_block_header_by_number(block_num, false).await;
        }

        self.proof_requests.fetch_add(1, Ordering::SeqCst);
        self.proof_gate.pass().await;
        if self.fail_proofs.load(Ordering::SeqCst) {
            return Err(RpcError::ConnectionError(Box::new(std::io::Error::other("offline"))));
        }

        let (header, proof) = self.inner.get_block_header_by_number(block_num, true).await?;
        let proof_forest = *self.proof_forest.lock().unwrap();
        let proof = match proof_forest {
            Some(forest) => Some(
                self.inner
                    .get_mmr()
                    .open_at(header.block_num().as_usize(), forest)
                    .expect("the proof forest includes the block"),
            ),
            None => proof,
        };
        Ok((header, proof))
    }

    async fn get_block_by_number(
        &self,
        block_num: BlockNumber,
        include_proof: bool,
    ) -> Result<(SignedBlock, Option<ExecutionProof>), RpcError> {
        self.inner.get_block_by_number(block_num, include_proof).await
    }

    async fn get_notes_by_id(&self, note_ids: &[NoteId]) -> Result<Vec<FetchedNote>, RpcError> {
        self.inner.get_notes_by_id(note_ids).await
    }

    async fn sync_chain_mmr(
        &self,
        current_block_height: BlockNumber,
        upper_bound: SyncTarget,
    ) -> Result<ChainMmrInfo, RpcError> {
        self.inner.sync_chain_mmr(current_block_height, upper_bound).await
    }

    async fn sync_notes(
        &self,
        block_from: BlockNumber,
        block_to: BlockNumber,
        note_tags: &BTreeSet<NoteTag>,
    ) -> Result<Vec<SyncNotesBlock>, RpcError> {
        let mut blocks = self.inner.sync_notes(block_from, block_to, note_tags).await?;
        let forest = Forest::new(block_to.as_usize() + 1).unwrap();
        for block in &mut blocks {
            block.mmr_path = self
                .inner
                .get_mmr()
                .open_at(block.block_header.block_num().as_usize(), forest)
                .unwrap()
                .merkle_path()
                .clone();
        }
        self.sync_notes_gate.pass().await;
        Ok(blocks)
    }

    async fn sync_nullifiers(
        &self,
        prefix: &[u16],
        block_from: BlockNumber,
        block_to: BlockNumber,
    ) -> Result<Vec<NullifierUpdate>, RpcError> {
        self.inner.sync_nullifiers(prefix, block_from, block_to).await
    }

    async fn get_account(
        &self,
        account_id: AccountId,
        request: GetAccountRequest,
    ) -> Result<(BlockNumber, AccountProof), RpcError> {
        self.inner.get_account(account_id, request).await
    }

    async fn register_account(
        &self,
        invitation_code: &str,
        account_id: AccountId,
    ) -> Result<(), RpcError> {
        self.inner.register_account(invitation_code, account_id).await
    }

    async fn is_account_allowed(&self, account_id: AccountId) -> Result<bool, RpcError> {
        self.inner.is_account_allowed(account_id).await
    }

    async fn is_invitation_code_valid(&self, invitation_code: &str) -> Result<bool, RpcError> {
        self.inner.is_invitation_code_valid(invitation_code).await
    }

    async fn get_note_script_by_root(&self, root: Word) -> Result<Option<NoteScript>, RpcError> {
        self.inner.get_note_script_by_root(root).await
    }

    async fn sync_storage_maps(
        &self,
        block_from: BlockNumber,
        block_to: BlockNumber,
        account_id: AccountId,
    ) -> Result<StorageMapInfo, RpcError> {
        self.inner.sync_storage_maps(block_from, block_to, account_id).await
    }

    async fn sync_account_vault(
        &self,
        block_from: BlockNumber,
        block_to: BlockNumber,
        account_id: AccountId,
    ) -> Result<AccountVaultInfo, RpcError> {
        self.inner.sync_account_vault(block_from, block_to, account_id).await
    }

    async fn sync_transactions(
        &self,
        block_from: BlockNumber,
        block_to: BlockNumber,
        account_ids: Vec<AccountId>,
    ) -> Result<Vec<TransactionRecord>, RpcError> {
        self.inner.sync_transactions(block_from, block_to, account_ids).await
    }

    async fn get_network_id(&self) -> Result<NetworkId, RpcError> {
        self.inner.get_network_id().await
    }

    async fn get_rpc_limits(&self) -> Result<RpcLimits, RpcError> {
        self.inner.get_rpc_limits().await
    }

    fn has_rpc_limits(&self) -> Option<RpcLimits> {
        self.inner.has_rpc_limits()
    }

    async fn set_rpc_limits(&self, limits: RpcLimits) {
        self.inner.set_rpc_limits(limits).await;
    }

    async fn get_status_unversioned(&self) -> Result<RpcStatusInfo, RpcError> {
        self.inner.get_status_unversioned().await
    }

    async fn get_network_note_status(
        &self,
        note_id: NoteId,
    ) -> Result<NetworkNoteStatusInfo, RpcError> {
        self.inner.get_network_note_status(note_id).await
    }
}

// HELPERS
// ================================================================================================

/// Builds a mock node whose chain commits the public note `notes[i]` in block `note_blocks[i]`.
///
/// The blocks must be ascending and larger than 0. The chain tip is the last block.
async fn node_with_notes(note_blocks: &[u32]) -> (Arc<MockRpcApi>, Vec<Note>) {
    let mut builder = MockChainBuilder::new();
    let account = builder.add_existing_mock_account(miden_testing::Auth::IncrNonce).unwrap();

    let mut notes = Vec::new();
    let mut spawn_notes = Vec::new();
    for seed in 0..note_blocks.len() {
        let note = NoteBuilder::new(account.id(), ChaCha20Rng::seed_from_u64(seed as u64))
            .note_type(NoteType::Public)
            .tag(NoteTag::new(0).into())
            .build()
            .unwrap();
        spawn_notes.push(builder.add_spawn_note(std::slice::from_ref(&note)).unwrap());
        notes.push(note);
    }

    let mut chain = builder.build().unwrap();
    for ((&block, note), spawn_note) in note_blocks.iter().zip(&notes).zip(spawn_notes) {
        while chain.latest_block_header().block_num().as_u32() + 1 < block {
            chain.prove_next_block().unwrap();
        }
        let transaction = Box::pin(
            chain
                .build_transaction(account.id())
                .unauthenticated_input_note(spawn_note)
                .expected_output_notes(vec![RawOutputNote::Full(note.clone())])
                .build()
                .unwrap()
                .execute(),
        )
        .await
        .unwrap();
        chain.add_pending_executed_transaction(&transaction).unwrap();
        chain.prove_next_block().unwrap();
        assert_eq!(chain.latest_block_header().block_num().as_u32(), block);
    }

    (Arc::new(MockRpcApi::new(chain)), notes)
}

/// Builds a client on the store at `store_path`.
async fn client_on(store_path: &Path, rpc: Arc<dyn NodeRpcClient>, cache_mmr: bool) -> Client {
    let mut client = ClientBuilder::new()
        .rpc(rpc)
        .sqlite_store(store_path.to_path_buf())
        .authenticator(Arc::new(FilesystemKeyStore::new(temp_dir()).unwrap()))
        .tx_discard_delta(None)
        .cache_partial_mmr_in_memory(cache_mmr)
        .build()
        .await
        .unwrap();
    client.ensure_genesis_in_place().await.unwrap();
    seed_mock_transaction_encryption_key(&mut client).await;
    client
}

/// Builds a sync of `client` that `gate` holds before it writes.
async fn gated_state_sync(
    client: &Client,
    rpc: Arc<dyn NodeRpcClient>,
    gate: Arc<Gate>,
) -> StateSync {
    StateSync::new(
        rpc,
        Arc::new(client.note_screener()),
        None,
        client.get_validator_config().await.unwrap(),
    )
    .with_note_observer(Arc::new(GateObserver(gate)))
}

/// Reads the partial MMR straight from the store, so that nothing repairs it.
async fn raw_partial_mmr(store_path: &Path) -> Result<PartialMmr, StoreError> {
    SqliteStore::new(store_path.to_path_buf())
        .await
        .unwrap()
        .get_current_partial_mmr()
        .await
}

/// Returns the number of siblings in the path of `block` at sync height `height`.
fn path_depth(height: BlockNumber, block: u32) -> usize {
    Forest::new(height.as_usize() + 1)
        .unwrap()
        .leaf_to_corresponding_tree(block as usize)
        .unwrap() as usize
}

/// Returns the index of the sibling at each level of the path of `block`, for `depth` levels.
fn sibling_indices(block: u32, depth: usize) -> Vec<InOrderIndex> {
    let mut index = InOrderIndex::from_leaf_pos(block as usize);
    let mut siblings = Vec::with_capacity(depth);
    for _ in 0..depth {
        siblings.push(index.sibling());
        index = index.parent();
    }
    siblings
}

/// Asserts that the store opens `block` with the full path that `node` has at `height`.
async fn assert_full_path(store_path: &Path, node: &MockRpcApi, block: u32, height: BlockNumber) {
    let partial_mmr = raw_partial_mmr(store_path).await.expect("the store must be readable");
    assert_eq!(partial_mmr.forest().num_leaves(), height.as_usize() + 1);
    let proof = partial_mmr.open(block as usize).unwrap().expect("the block must be tracked");
    let expected = node
        .get_mmr()
        .open_at(block as usize, Forest::new(height.as_usize() + 1).unwrap())
        .unwrap();
    assert_eq!(usize::from(proof.merkle_path().depth()), path_depth(height, block));
    assert_eq!(proof.merkle_path(), expected.merkle_path());
}

fn delete_nodes(store_path: &Path, indices: &[InOrderIndex]) {
    let conn = rusqlite::Connection::open(store_path).unwrap();
    for index in indices {
        let deleted = conn
            .execute(
                "DELETE FROM partial_blockchain_nodes WHERE id = ?1",
                [i64::try_from(index.inner()).unwrap()],
            )
            .unwrap();
        assert_eq!(deleted, 1, "node {} must be stored before the test deletes it", index.inner());
    }
}

fn replace_node(store_path: &Path, index: InOrderIndex, value: Word) {
    let conn = rusqlite::Connection::open(store_path).unwrap();
    let updated = conn
        .execute(
            "UPDATE partial_blockchain_nodes SET node = ?1 WHERE id = ?2",
            rusqlite::params![value.to_bytes(), i64::try_from(index.inner()).unwrap()],
        )
        .unwrap();
    assert_eq!(updated, 1);
}

async fn stored_nodes(store_path: &Path) -> BTreeMap<InOrderIndex, Word> {
    SqliteStore::new(store_path.to_path_buf())
        .await
        .unwrap()
        .get_partial_blockchain_nodes(PartialBlockchainFilter::All)
        .await
        .unwrap()
}

fn is_stale_write(result: &Result<impl core::fmt::Debug, ClientError>) -> bool {
    matches!(result, Err(ClientError::StoreError(StoreError::DatabaseTransientError(_))))
}

fn is_inconsistent_mmr(result: &Result<PartialMmr, ClientError>) -> bool {
    matches!(
        result,
        Err(ClientError::StoreError(StoreError::MmrError(MmrError::InconsistentPartialMmr(
            _
        ))))
    )
}

/// A store at sync height [`HEAL_HEIGHT`] that tracks each block of `note_blocks` with its full
/// path, and the client that wrote it.
struct HealSetup {
    client: Client,
    rpc: Arc<GatedRpc>,
    node: Arc<MockRpcApi>,
    store_path: PathBuf,
}

async fn heal_setup(note_blocks: &[u32]) -> HealSetup {
    let (node, notes) = node_with_notes(note_blocks).await;
    node.advance_blocks(HEAL_HEIGHT - node.get_chain_tip_block_num().as_u32());
    let rpc = GatedRpc::new(node.clone());
    let store_path = create_test_store_path();
    let mut client = client_on(&store_path, rpc.clone(), false).await;
    client.sync_state().await.unwrap();
    let note_files: Vec<NoteFile> = notes.iter().map(|note| NoteFile::NoteId(note.id())).collect();
    client.import_notes(&note_files).await.unwrap();

    let height = client.get_sync_height().await.unwrap();
    assert_eq!(height, BlockNumber::from(HEAL_HEIGHT));
    for &block in note_blocks {
        assert_full_path(&store_path, &node, block, height).await;
    }

    HealSetup { client, rpc, node, store_path }
}

// RACE TESTS
// ================================================================================================

/// An import by proof that waits on its proof request while another client syncs across a tree
/// merge stores the full path at the new sync height.
#[tokio::test]
async fn import_by_proof_across_a_sync_stores_the_full_path() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let rpc = GatedRpc::new(node.clone());
    let store_path = create_test_store_path();
    let mut importer = client_on(&store_path, rpc.clone(), false).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;

    importer.sync_state().await.unwrap();
    let height_before = importer.get_sync_height().await.unwrap();

    rpc.proof_gate.arm();
    let note_files = [NoteFile::NoteId(notes[0].id())];
    let (import, height_after) =
        Box::pin(while_held(&rpc.proof_gate, importer.import_notes(&note_files), async {
            node.advance_blocks(40);
            syncer.sync_state().await.unwrap();
            syncer.get_sync_height().await.unwrap()
        }))
        .await;
    let height_after = height_after.expect("the import must request the block proof");
    assert!(path_depth(height_after, NOTE_BLOCK) > path_depth(height_before, NOTE_BLOCK));

    import.expect("the import must succeed");
    assert_full_path(&store_path, &node, NOTE_BLOCK, height_after).await;
    assert_eq!(
        node.block_header_requests(NOTE_BLOCK.into()),
        [true],
        "the import must trim the proof it holds again instead of requesting a new one"
    );
}

/// An import by details whose note sync spans a sync of another client across a tree merge stores
/// the full path at the new sync height.
#[tokio::test]
async fn import_by_details_across_a_sync_stores_the_full_path() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let rpc = GatedRpc::new(node.clone());
    let store_path = create_test_store_path();
    let mut importer = client_on(&store_path, rpc.clone(), false).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;

    importer.sync_state().await.unwrap();
    let height_before = importer.get_sync_height().await.unwrap();

    rpc.sync_notes_gate.arm();
    let note_files = [NoteFile::ExpectedNote {
        details: notes[0].clone().into(),
        sync_hint: NoteSyncHint::new(BlockNumber::GENESIS, notes[0].metadata().tag()),
    }];
    let (import, height_after) =
        Box::pin(while_held(&rpc.sync_notes_gate, importer.import_notes(&note_files), async {
            node.advance_blocks(40);
            syncer.sync_state().await.unwrap();
            syncer.get_sync_height().await.unwrap()
        }))
        .await;
    let height_after = height_after.expect("the import must sync the notes");
    assert!(path_depth(height_after, NOTE_BLOCK) > path_depth(height_before, NOTE_BLOCK));

    import.expect("the import must succeed");
    assert_full_path(&store_path, &node, NOTE_BLOCK, height_after).await;
}

/// A sync that crosses the tree merge of a block another client tracked after the sync read the
/// partial MMR is refused. The next sync stores the full path.
#[tokio::test]
async fn sync_across_an_import_is_refused_then_retried() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let store_path = create_test_store_path();
    let mut syncer = client_on(&store_path, node.clone(), false).await;
    let mut importer = client_on(&store_path, node.clone(), false).await;

    syncer.sync_state().await.unwrap();
    let height_before = syncer.get_sync_height().await.unwrap();
    node.advance_blocks(40);

    let gate = Arc::new(Gate::default());
    gate.arm();
    let state_sync = gated_state_sync(&syncer, node.clone(), gate.clone()).await;
    let mut chain_sync_data = syncer.fetch_chain_updates(&state_sync).await.unwrap();
    state_sync.derive_state_updates(&mut chain_sync_data).await.unwrap();
    state_sync.fetch_nullifiers(&mut chain_sync_data).await.unwrap();

    let note_files = [NoteFile::NoteId(notes[0].id())];
    let (apply, import) = Box::pin(while_held(
        &gate,
        syncer.apply_chain_updates(&state_sync, chain_sync_data),
        importer.import_notes(&note_files),
    ))
    .await;
    import.expect("the sync must reach its hook").expect("the import must succeed");
    assert!(is_stale_write(&apply), "the stale sync must be refused, got {apply:?}");

    // The refused sync wrote nothing.
    assert_eq!(syncer.get_sync_height().await.unwrap(), height_before);
    assert_full_path(&store_path, &node, NOTE_BLOCK, height_before).await;

    syncer.sync_chain().await.unwrap();
    let height_after = syncer.get_sync_height().await.unwrap();
    assert!(path_depth(height_after, NOTE_BLOCK) > path_depth(height_before, NOTE_BLOCK));
    assert_full_path(&store_path, &node, NOTE_BLOCK, height_after).await;
}

/// A sync that starts from a sync height another client already moved is refused and writes
/// nothing.
#[tokio::test]
async fn sync_from_a_stale_sync_height_is_refused() {
    let (node, _notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let store_path = create_test_store_path();
    let mut stale = client_on(&store_path, node.clone(), false).await;
    let mut other = client_on(&store_path, node.clone(), false).await;

    stale.sync_state().await.unwrap();
    node.advance_blocks(40);

    let gate = Arc::new(Gate::default());
    gate.arm();
    let state_sync = gated_state_sync(&stale, node.clone(), gate.clone()).await;
    let mut chain_sync_data = stale.fetch_chain_updates(&state_sync).await.unwrap();
    state_sync.derive_state_updates(&mut chain_sync_data).await.unwrap();
    state_sync.fetch_nullifiers(&mut chain_sync_data).await.unwrap();

    let store = SqliteStore::new(store_path.clone()).await.unwrap();
    let (apply, checkpoint) = Box::pin(while_held(
        &gate,
        stale.apply_chain_updates(&state_sync, chain_sync_data),
        async {
            other.sync_chain().await.unwrap();
            (
                store.get_sync_height().await.unwrap(),
                store.get_current_blockchain_peaks().await.unwrap(),
            )
        },
    ))
    .await;
    let (height, peaks) = checkpoint.expect("the sync must reach its hook");

    assert!(is_stale_write(&apply), "the stale sync must be refused, got {apply:?}");
    assert_eq!(store.get_sync_height().await.unwrap(), height);
    assert_eq!(store.get_current_blockchain_peaks().await.unwrap(), peaks);
}

/// A client that caches its partial MMR sees a block another client tracked while its own sync was
/// in progress, and a later sync across the tree merge of that block stores the full path.
#[tokio::test]
async fn cached_partial_mmr_sees_a_block_tracked_during_a_sync() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let store_path = create_test_store_path();
    let mut cached = client_on(&store_path, node.clone(), true).await;
    let mut importer = client_on(&store_path, node.clone(), false).await;

    // At forests 41 and 51 leaf 1 is in the tree of 32 leaves. At forest 71 it is in the tree of 64
    // leaves.
    node.advance_blocks(39);
    cached.sync_state().await.unwrap();
    assert_eq!(cached.get_sync_height().await.unwrap(), BlockNumber::from(40u32));
    node.advance_blocks(10);

    let gate = Arc::new(Gate::default());
    gate.arm();
    let state_sync = gated_state_sync(&cached, node.clone(), gate.clone()).await;
    let mut chain_sync_data = cached.fetch_chain_updates(&state_sync).await.unwrap();
    state_sync.derive_state_updates(&mut chain_sync_data).await.unwrap();
    state_sync.fetch_nullifiers(&mut chain_sync_data).await.unwrap();

    let note_files = [NoteFile::NoteId(notes[0].id())];
    let (apply, import) = Box::pin(while_held(
        &gate,
        cached.apply_chain_updates(&state_sync, chain_sync_data),
        importer.import_notes(&note_files),
    ))
    .await;
    import.expect("the sync must reach its hook").expect("the import must succeed");
    apply.expect("a sync that keeps the tracked path complete must succeed");

    let partial_mmr = cached.get_current_partial_mmr().await.unwrap();
    assert!(
        partial_mmr.is_tracked(NOTE_BLOCK as usize),
        "the client must not use a cached partial MMR that lacks a tracked block"
    );

    node.advance_blocks(20);
    cached.sync_chain().await.unwrap();
    let height = cached.get_sync_height().await.unwrap();
    assert_eq!(height, BlockNumber::from(HEAL_HEIGHT));
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;
}

// HEAL TESTS
// ================================================================================================

/// A store that lacks the upper siblings of a tracked block, as in the field report, is repaired
/// with one proof request.
#[tokio::test]
async fn partial_mmr_read_heals_missing_upper_siblings() {
    let HealSetup { mut client, rpc, node, store_path } = heal_setup(&[NOTE_BLOCK]).await;
    let height = BlockNumber::from(HEAL_HEIGHT);
    let siblings = sibling_indices(NOTE_BLOCK, path_depth(height, NOTE_BLOCK));
    delete_nodes(&store_path, &siblings[2..]);

    let raw = raw_partial_mmr(&store_path).await;
    assert!(
        matches!(raw, Err(StoreError::MmrError(MmrError::InconsistentPartialMmr(_)))),
        "the test must start from a store that cannot be read, got {raw:?}"
    );

    let requests_before = rpc.proof_requests();
    let healed = client.get_current_partial_mmr().await;
    healed.expect("the read must repair the store");
    assert_eq!(rpc.proof_requests(), requests_before + 1);
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;

    client.get_current_partial_mmr().await.unwrap();
    assert_eq!(rpc.proof_requests(), requests_before + 1, "a readable store needs no request");

    client.sync_state().await.unwrap();
}

/// A store that lacks the leaf of a tracked block and one lower sibling is repaired.
#[tokio::test]
async fn partial_mmr_read_heals_missing_leaf_and_lower_sibling() {
    let HealSetup { client, node, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
    let height = BlockNumber::from(HEAL_HEIGHT);
    let siblings = sibling_indices(NOTE_BLOCK, path_depth(height, NOTE_BLOCK));
    delete_nodes(&store_path, &[InOrderIndex::from_leaf_pos(NOTE_BLOCK as usize), siblings[1]]);
    assert!(raw_partial_mmr(&store_path).await.is_err());

    client.get_current_partial_mmr().await.expect("the read must repair the store");
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;
}

/// A store with two tracked blocks that each lack different nodes is repaired.
#[tokio::test]
async fn partial_mmr_read_heals_two_broken_blocks() {
    // Blocks 1 and 6 share their siblings at levels 3 and higher, so the test removes nodes at
    // levels 0 and 1, which only one of the blocks uses.
    let blocks = [NOTE_BLOCK, 6];
    let HealSetup { client, node, store_path, .. } = heal_setup(&blocks).await;
    let height = BlockNumber::from(HEAL_HEIGHT);
    for block in blocks {
        delete_nodes(&store_path, &sibling_indices(block, 2));
    }
    assert!(raw_partial_mmr(&store_path).await.is_err());

    client.get_current_partial_mmr().await.expect("the read must repair the store");
    for block in blocks {
        assert_full_path(&store_path, &node, block, height).await;
    }
}

/// When the node cannot be reached, the read returns the original error and writes nothing. The
/// next read after the node is reachable again repairs the store.
#[tokio::test]
async fn partial_mmr_heal_without_the_node_writes_nothing() {
    let HealSetup { client, rpc, node, store_path } = heal_setup(&[NOTE_BLOCK]).await;
    let height = BlockNumber::from(HEAL_HEIGHT);
    let siblings = sibling_indices(NOTE_BLOCK, path_depth(height, NOTE_BLOCK));
    delete_nodes(&store_path, &siblings[2..]);
    let nodes_before = stored_nodes(&store_path).await;

    rpc.fail_proofs.store(true, Ordering::SeqCst);
    let requests_before = rpc.proof_requests();
    let result = client.get_current_partial_mmr().await;
    assert!(
        is_inconsistent_mmr(&result),
        "the read must return the original error, got {result:?}"
    );
    assert_eq!(rpc.proof_requests(), requests_before + 1);
    assert_eq!(stored_nodes(&store_path).await, nodes_before);

    rpc.fail_proofs.store(false, Ordering::SeqCst);
    client.get_current_partial_mmr().await.expect("the read must repair the store");
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;
}

/// A node whose proof is for a smaller forest than the store cannot repair the store, and the read
/// writes nothing.
#[tokio::test]
async fn partial_mmr_heal_with_a_lagging_node_writes_nothing() {
    let HealSetup { client, rpc, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
    let height = BlockNumber::from(HEAL_HEIGHT);
    let siblings = sibling_indices(NOTE_BLOCK, path_depth(height, NOTE_BLOCK));
    delete_nodes(&store_path, &siblings[2..]);
    let nodes_before = stored_nodes(&store_path).await;

    *rpc.proof_forest.lock().unwrap() = Some(Forest::new(40).unwrap());
    let requests_before = rpc.proof_requests();
    let result = client.get_current_partial_mmr().await;
    assert!(
        is_inconsistent_mmr(&result),
        "the read must return the original error, got {result:?}"
    );
    assert_eq!(rpc.proof_requests(), requests_before + 1);
    assert_eq!(stored_nodes(&store_path).await, nodes_before);
}

/// A stored node that differs from the node's proof stops the repair, and the read writes nothing.
#[tokio::test]
async fn partial_mmr_heal_refuses_a_conflicting_stored_node() {
    let HealSetup { client, rpc, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
    let height = BlockNumber::from(HEAL_HEIGHT);
    let siblings = sibling_indices(NOTE_BLOCK, path_depth(height, NOTE_BLOCK));
    delete_nodes(&store_path, &siblings[2..]);
    replace_node(&store_path, siblings[0], Word::new([Felt::from(7u32); 4]));
    let nodes_before = stored_nodes(&store_path).await;

    let requests_before = rpc.proof_requests();
    let result = client.get_current_partial_mmr().await;
    assert!(
        is_inconsistent_mmr(&result),
        "the read must return the original error, got {result:?}"
    );
    assert_eq!(rpc.proof_requests(), requests_before + 1);
    assert_eq!(stored_nodes(&store_path).await, nodes_before);
}

/// A block that another client untracks while the repair waits for its proof stays untracked.
#[tokio::test]
async fn partial_mmr_heal_does_not_track_an_untracked_block_again() {
    let HealSetup { client, rpc, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
    let height = BlockNumber::from(HEAL_HEIGHT);
    let siblings = sibling_indices(NOTE_BLOCK, path_depth(height, NOTE_BLOCK));
    delete_nodes(&store_path, &siblings[2..]);
    let other_store = SqliteStore::new(store_path.clone()).await.unwrap();

    rpc.proof_gate.arm();
    let (result, untrack) = while_held(
        &rpc.proof_gate,
        client.get_current_partial_mmr(),
        other_store.untrack_and_prune_irrelevant_blocks(&[NOTE_BLOCK.into()], &[]),
    )
    .await;
    untrack.expect("the read must request the block proof").unwrap();

    let partial_mmr = result.expect("the read must succeed");
    assert!(!partial_mmr.is_tracked(NOTE_BLOCK as usize));
    let tracked = other_store.get_tracked_block_header_numbers().await.unwrap();
    assert!(!tracked.contains(&(NOTE_BLOCK as usize)), "the repair must not track the block");
    raw_partial_mmr(&store_path).await.unwrap();
}
