//! Two client instances on one store, and a store that already lacks part of a tracked path.
//!
//! The race tests hold one client at a point between its read of the partial MMR and its write, let
//! the other client change the chain data, then let the first client continue. The heal tests
//! remove stored nodes with raw SQL, as an older client could leave them. The refusal tests use a
//! store that refuses chain writes, so that each branch that handles a refused write runs.

use std::collections::{BTreeMap, BTreeSet};
use std::env::temp_dir;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use miden_client::builder::ClientBuilder;
use miden_client::crypto::{Forest, InOrderIndex, PartialMmr};
use miden_client::keystore::FilesystemKeyStore;
use miden_client::note::{BlockNumber, NoteDetails, NoteFile, NoteSyncHint};
use miden_client::rpc::domain::account::{AccountProof, GetAccountRequest};
use miden_client::rpc::domain::account_vault::AccountVaultInfo;
use miden_client::rpc::domain::note::{CommittedNote, FetchedNote, SyncNotesBlock, SyncedNote};
use miden_client::rpc::domain::nullifier::NullifierUpdate;
use miden_client::rpc::domain::storage_map::StorageMapInfo;
use miden_client::rpc::domain::sync::{ChainMmrInfo, SyncTarget};
use miden_client::rpc::domain::transaction::TransactionRecord;
use miden_client::rpc::encryption::{AttestedTransactionEncryptionKey, SealedTransactionInputs};
use miden_client::rpc::{NetworkNoteStatusInfo, NodeRpcClient, RpcError, RpcLimits, RpcStatusInfo};
use miden_client::store::input_note_states::{CommittedNoteState, ExpectedNoteState};
use miden_client::store::{
    InputNoteRecord,
    InputNoteState,
    NoteFilter,
    PartialBlockchainFilter,
    Store,
    StoreError,
};
use miden_client::sync::{NoteObserver, NoteTagRecord, StateSync, StateSyncUpdate};
use miden_client::testing::common::{TestClient, create_test_store_path};
use miden_client::testing::mock::{MockClient, MockRpcApi};
use miden_client::testing::note_transport::{MockNoteTransportApi, MockNoteTransportNode};
use miden_client::transaction::{PswapTransactionData, TransactionRequestBuilder};
use miden_client::utils::{RwLock, Serializable};
use miden_client::{ClientError, Word};
use miden_client_sqlite_store::{ClientBuilderSqliteExt, SqliteStore};
use miden_protocol::account::{AccountId, AccountType};
use miden_protocol::address::NetworkId;
use miden_protocol::asset::{AssetAmount, FungibleAsset};
use miden_protocol::batch::{ProposedBatch, ProvenBatch};
use miden_protocol::block::{BlockHeader, SignedBlock};
use miden_protocol::crypto::merkle::mmr::{MmrError, MmrProof};
use miden_protocol::crypto::merkle::smt::{LeafIndex, SimpleSmt};
use miden_protocol::note::{Note, NoteId, NoteInclusionProof, NoteScript, NoteTag, NoteType};
use miden_protocol::transaction::{ExecutedTransaction, ProvenTransaction, RawOutputNote};
use miden_protocol::vm::ExecutionProof;
use miden_protocol::{BLOCK_NOTE_TREE_DEPTH, Felt};
use miden_standards::note::{PswapNote, PswapNoteAttachment};
use miden_standards::testing::note::NoteBuilder;
use miden_testing::MockChainBuilder;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use refusing_store::RefusingStore;

use crate::tests::{
    create_prebuilt_mock_chain,
    create_pswap_test_client,
    seed_mock_transaction_encryption_key,
};

mod refusing_store;

type Client = MockClient<FilesystemKeyStore>;

/// The block that commits the note in the single-note tests.
const NOTE_BLOCK: u32 = 1;

/// The sync height of the heal tests. At forest 71, leaf 1 is in the tree of 64 leaves and its path
/// has 6 siblings.
const HEAL_HEIGHT: u32 = 70;

/// The number of times a sync fetches and writes the chain updates while the store refuses the
/// write.
const STALE_WRITE_ATTEMPTS: usize = 3;

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

/// Delegates to the mock node, with switches for the block proof requests, the note sync and the
/// chain sync.
///
/// `sync_notes` returns each path at the forest of `block_to`, as the node does. The mock node
/// returns the paths at its current tip instead.
struct GatedRpc {
    inner: Arc<MockRpcApi>,
    /// Holds a block header request that includes an MMR proof.
    proof_gate: Gate,
    /// Holds a note sync request.
    sync_notes_gate: Gate,
    /// Holds a chain MMR sync request.
    chain_gate: Gate,
    /// Makes each block proof request fail as if the node were not reachable.
    fail_proofs: AtomicBool,
    /// Returns each block proof at this forest, as a node that is behind the client does.
    proof_forest: Mutex<Option<Forest>>,
    /// Returns the next block proof at this forest, once.
    next_proof_forest: Mutex<Option<Forest>>,
    /// Returns each block of a note sync with the header and the note paths that
    /// [`forge_note_block`] makes.
    forge_note_blocks: AtomicBool,
    /// Returns each block proof with a header that is not in the chain, and a proof whose leaf is
    /// that header.
    forge_proofs: AtomicBool,
    /// The number of block proof requests, including the failed ones.
    proof_requests: AtomicUsize,
    /// The number of chain MMR sync requests.
    chain_requests: AtomicUsize,
}

impl GatedRpc {
    fn new(inner: Arc<MockRpcApi>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            proof_gate: Gate::default(),
            sync_notes_gate: Gate::default(),
            chain_gate: Gate::default(),
            fail_proofs: AtomicBool::new(false),
            proof_forest: Mutex::new(None),
            next_proof_forest: Mutex::new(None),
            forge_note_blocks: AtomicBool::new(false),
            forge_proofs: AtomicBool::new(false),
            proof_requests: AtomicUsize::new(0),
            chain_requests: AtomicUsize::new(0),
        })
    }

    fn proof_requests(&self) -> usize {
        self.proof_requests.load(Ordering::SeqCst)
    }

    fn chain_requests(&self) -> usize {
        self.chain_requests.load(Ordering::SeqCst)
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
        let next_proof_forest = self.next_proof_forest.lock().unwrap().take();
        let proof_forest = next_proof_forest.or(*self.proof_forest.lock().unwrap());
        let proof = match proof_forest {
            Some(forest) => Some(
                self.inner
                    .get_mmr()
                    .open_at(header.block_num().as_usize(), forest)
                    .expect("the proof forest includes the block"),
            ),
            None => proof,
        };
        if self.forge_proofs.load(Ordering::SeqCst) {
            let forged = forged_header(&header, header.note_root(), header.timestamp() + 1);
            let proof = proof.map(|proof| MmrProof::new(proof.path().clone(), forged.commitment()));
            return Ok((forged, proof));
        }
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
        self.chain_requests.fetch_add(1, Ordering::SeqCst);
        self.chain_gate.pass().await;
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
            if self.forge_note_blocks.load(Ordering::SeqCst) {
                forge_note_block(block);
            }
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

/// Returns `header` with `note_root` and `timestamp` in place of its own values.
fn forged_header(header: &BlockHeader, note_root: Word, timestamp: u32) -> BlockHeader {
    BlockHeader::new(
        header.prev_block_commitment(),
        header.block_num(),
        header.chain_commitment(),
        header.account_root(),
        header.nullifier_root(),
        note_root,
        header.tx_commitment(),
        header.validator_config().clone(),
        header.fee_parameters().clone(),
        header.protocol_config_commitment(),
        header.next_protocol_config().cloned(),
        timestamp,
    )
}

/// Replaces the header of `block` with a header whose note tree also holds a note that the chain
/// does not have, and replaces the inclusion proof of each note with its path in that tree.
///
/// Each path verifies against the note root of the new header, but the chain MMR does not have the
/// new header.
fn forge_note_block(block: &mut SyncNotesBlock) {
    let block_num = block.block_header.block_num();
    let mut leaves: Vec<(u64, Word)> = block
        .notes
        .values()
        .map(|note| {
            let index = note.inclusion_proof().location().block_note_tree_index();
            (u64::from(index), note.note_id().as_word())
        })
        .collect();
    let unknown_index = leaves.iter().map(|(index, _)| index + 1).max().expect("a note block");
    leaves.push((unknown_index, Word::new([Felt::from(9u32); 4])));
    let tree = SimpleSmt::<BLOCK_NOTE_TREE_DEPTH>::with_leaves(leaves).unwrap();

    for note in block.notes.values_mut() {
        let index = note.inclusion_proof().location().block_note_tree_index();
        let path = tree.open(&LeafIndex::new(u64::from(index)).unwrap()).path;
        let proof = NoteInclusionProof::new(block_num, index, path).unwrap();
        let mut forged = CommittedNote::new(*note.note_id(), *note.metadata(), proof);
        if let Some(attachments) = note.attachments() {
            forged = forged.with_attachments(attachments.clone()).unwrap();
        }
        *note = forged;
    }

    let timestamp = block.block_header.timestamp();
    block.block_header = forged_header(&block.block_header, tree.root(), timestamp);
}

// HELPERS
// ================================================================================================

/// Builds a mock node whose chain commits the public note `notes[i]` in block `note_blocks[i]`.
///
/// The blocks must be ascending and larger than 0. The chain tip is the last block.
async fn node_with_notes(note_blocks: &[u32]) -> (Arc<MockRpcApi>, Vec<Note>) {
    let (node, notes, _) = build_node(note_blocks, false).await;
    (node, notes)
}

/// Builds a mock node as [`node_with_notes`] does, and a note that the node commits only when the
/// test commits it.
async fn node_with_a_late_note(note_blocks: &[u32]) -> (Arc<MockRpcApi>, Vec<Note>, LateNote) {
    let (node, notes, late_note) = build_node(note_blocks, true).await;
    (node, notes, late_note.expect("the node must have a late note"))
}

/// A public note whose transaction is executed, but which the node commits only when a test commits
/// it.
struct LateNote {
    note: Note,
    transaction: ExecutedTransaction,
}

impl LateNote {
    /// Commits the note in a new block of `node`, and returns that block.
    fn commit(&self, node: &MockRpcApi) -> BlockNumber {
        node.add_pending_executed_transaction(&self.transaction);
        node.prove_block();
        node.get_chain_tip_block_num()
    }
}

async fn build_node(
    note_blocks: &[u32],
    with_late_note: bool,
) -> (Arc<MockRpcApi>, Vec<Note>, Option<LateNote>) {
    let mut builder = MockChainBuilder::new();
    let account = builder.add_existing_mock_account(miden_testing::Auth::IncrNonce).unwrap();

    let mut notes = Vec::new();
    let mut spawn_notes = Vec::new();
    for seed in 0..note_blocks.len() + usize::from(with_late_note) {
        let note = NoteBuilder::new(account.id(), ChaCha20Rng::seed_from_u64(seed as u64))
            .note_type(NoteType::Public)
            .tag(NoteTag::new(0).into())
            .build()
            .unwrap();
        spawn_notes.push(builder.add_spawn_note(std::slice::from_ref(&note)).unwrap());
        notes.push(note);
    }

    let mut chain = builder.build().unwrap();
    let mut late_note = None;
    for (index, (note, spawn_note)) in notes.iter().zip(spawn_notes).enumerate() {
        let block = note_blocks.get(index).copied();
        if let Some(block) = block {
            while chain.latest_block_header().block_num().as_u32() + 1 < block {
                chain.prove_next_block().unwrap();
            }
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
        let Some(block) = block else {
            late_note = Some(LateNote { note: note.clone(), transaction });
            break;
        };
        chain.add_pending_executed_transaction(&transaction).unwrap();
        chain.prove_next_block().unwrap();
        assert_eq!(chain.latest_block_header().block_num().as_u32(), block);
    }
    notes.truncate(note_blocks.len());

    (Arc::new(MockRpcApi::new(chain)), notes, late_note)
}

/// Builds a client on the store at `store_path`.
async fn client_on(store_path: &Path, rpc: Arc<dyn NodeRpcClient>, cache_mmr: bool) -> Client {
    build_client(
        ClientBuilder::new()
            .rpc(rpc)
            .sqlite_store(store_path.to_path_buf())
            .cache_partial_mmr_in_memory(cache_mmr),
    )
    .await
}

/// Builds a client on a [`RefusingStore`] over a new store file. `configure` sets the other options
/// that the test needs.
async fn refusing_client(
    rpc: Arc<dyn NodeRpcClient>,
    configure: impl FnOnce(ClientBuilder<FilesystemKeyStore>) -> ClientBuilder<FilesystemKeyStore>,
) -> (Client, Arc<RefusingStore>, PathBuf) {
    let store_path = create_test_store_path();
    let store = Arc::new(RefusingStore::new(&store_path).await);
    let client = build_client(configure(ClientBuilder::new().rpc(rpc).store(store.clone()))).await;
    (client, store, store_path)
}

/// Builds the client that `builder` configures, with the options that all test clients share.
async fn build_client(builder: ClientBuilder<FilesystemKeyStore>) -> Client {
    let mut client = builder
        .authenticator(Arc::new(FilesystemKeyStore::new(temp_dir()).unwrap()))
        .tx_discard_delta(None)
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

/// Deletes the siblings of `block` above level 1 at [`HEAL_HEIGHT`], as in the field report.
fn delete_upper_siblings(store_path: &Path, block: u32) {
    let siblings = sibling_indices(block, path_depth(BlockNumber::from(HEAL_HEIGHT), block));
    delete_nodes(store_path, &siblings[2..]);
}

/// Marks `block` as not tracked and keeps its header.
fn untrack_with_sql(store_path: &Path, block: u32) {
    let conn = rusqlite::Connection::open(store_path).unwrap();
    let updated = conn
        .execute("UPDATE block_headers SET has_client_notes = 0 WHERE block_num = ?1", [block])
        .unwrap();
    assert_eq!(updated, 1);
}

/// Stores the header of `block` as tracked, with its leaf and its full path at `height`, as an
/// import of another note in that block does.
async fn track_with_full_path(
    store_path: &Path,
    node: &MockRpcApi,
    block: u32,
    height: BlockNumber,
) {
    let (header, _) = node.get_block_header_by_number(Some(block.into()), false).await.unwrap();
    let proof = node
        .get_mmr()
        .open_at(block as usize, Forest::new(height.as_usize() + 1).unwrap())
        .unwrap();
    let mut nodes = vec![(InOrderIndex::from_leaf_pos(block as usize), header.commitment())];
    nodes.extend(
        sibling_indices(block, path_depth(height, block))
            .into_iter()
            .zip(proof.merkle_path().nodes().iter().copied()),
    );
    SqliteStore::new(store_path.to_path_buf())
        .await
        .unwrap()
        .insert_block_header(&header, &nodes, true)
        .await
        .unwrap();
}

/// Deletes the header of `block`, as a prune of another client does.
fn delete_header_with_sql(store_path: &Path, block: u32) {
    let conn = rusqlite::Connection::open(store_path).unwrap();
    let deleted = conn.execute("DELETE FROM block_headers WHERE block_num = ?1", [block]).unwrap();
    assert_eq!(deleted, 1);
}

/// Returns the leaf of `block` and its siblings at `height`.
fn leaf_and_path_indices(block: u32, height: BlockNumber) -> Vec<InOrderIndex> {
    let mut indices = vec![InOrderIndex::from_leaf_pos(block as usize)];
    indices.extend(sibling_indices(block, path_depth(height, block)));
    indices
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

/// Returns whether `result` is the error of a stale chain write, which the store and the client
/// return when another writer changed the chain data.
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

/// Returns whether `result` is the error of a partial MMR read that stops at the tracked block
/// `block`, because a node of its path is missing.
fn is_missing_path_of(result: &Result<PartialMmr, ClientError>, block: u32) -> bool {
    matches!(
        result,
        Err(ClientError::StoreError(StoreError::MmrError(MmrError::InconsistentPartialMmr(
            message
        )))) if message.ends_with(&format!("tracked leaf at position {block}"))
    )
}

/// A store at sync height [`HEAL_HEIGHT`] that tracks each block of `note_blocks` with its full
/// path, and the client that wrote it.
struct HealSetup {
    client: Client,
    rpc: Arc<GatedRpc>,
    node: Arc<MockRpcApi>,
    store: Arc<RefusingStore>,
    store_path: PathBuf,
}

async fn heal_setup(note_blocks: &[u32]) -> HealSetup {
    let (node, notes) = node_with_notes(note_blocks).await;
    heal_setup_on(node, &notes, note_blocks).await
}

/// Builds the [`HealSetup`] on `node`, which commits `notes[i]` in block `note_blocks[i]`.
async fn heal_setup_on(node: Arc<MockRpcApi>, notes: &[Note], note_blocks: &[u32]) -> HealSetup {
    node.advance_blocks(HEAL_HEIGHT - node.get_chain_tip_block_num().as_u32());
    let rpc = GatedRpc::new(node.clone());
    let (mut client, store, store_path) = refusing_client(rpc.clone(), |builder| builder).await;
    client.sync_state().await.unwrap();
    let note_files: Vec<NoteFile> = notes.iter().map(|note| NoteFile::NoteId(note.id())).collect();
    client.import_notes(&note_files).await.unwrap();

    let height = client.get_sync_height().await.unwrap();
    assert_eq!(height, BlockNumber::from(HEAL_HEIGHT));
    for &block in note_blocks {
        assert_full_path(&store_path, &node, block, height).await;
    }

    HealSetup { client, rpc, node, store, store_path }
}

/// Stores `expected` as an expected note with its metadata and tracks its note tag, as a client
/// does for a note that its own transaction creates. Returns the note tag.
async fn store_expected_note(store: &RefusingStore, expected: &Note) -> NoteTagRecord {
    let metadata = *expected.metadata();
    let state = ExpectedNoteState {
        metadata: Some(metadata),
        after_block_num: BlockNumber::GENESIS,
        tag: Some(metadata.tag()),
    };
    let record = InputNoteRecord::new(
        NoteDetails::from(expected.clone()),
        expected.attachments().clone(),
        None,
        state.into(),
    );
    store.upsert_input_notes(&[record]).await.unwrap();
    let tag = NoteTagRecord::with_note_source(metadata.tag(), expected.details_commitment());
    store.add_note_tag(tag).await.unwrap();
    tag
}

async fn has_note_tag(client: &Client, tag: NoteTagRecord) -> bool {
    client.get_note_tags().await.unwrap().contains(&tag)
}

/// Returns the file that imports `expected` by its details, with a sync hint from the genesis
/// block.
fn expected_note_file(expected: &Note) -> NoteFile {
    NoteFile::ExpectedNote {
        details: expected.clone().into(),
        sync_hint: NoteSyncHint::new(BlockNumber::GENESIS, expected.metadata().tag()),
    }
}

/// Spends `note` in a new block of `node`, and returns that block.
async fn spend_note(node: &MockRpcApi, note: &Note) -> BlockNumber {
    let chain = node.mock_chain.read().clone();
    let transaction = Box::pin(
        chain
            .build_transaction(node.first_account_id())
            .authenticated_input_note(note.id())
            .build()
            .unwrap()
            .execute(),
    )
    .await
    .unwrap();
    node.add_pending_executed_transaction(&transaction);
    node.prove_block();
    node.get_chain_tip_block_num()
}

/// Returns the committed record of the public note `note_id`, as a writer that verifies the note
/// against the header of its block stores it.
async fn committed_record(node: &MockRpcApi, note_id: NoteId) -> InputNoteRecord {
    let Some(FetchedNote::Public(public_note, inclusion_proof)) =
        node.get_notes_by_id(&[note_id]).await.unwrap().pop()
    else {
        panic!("the node must return the public note");
    };
    let block_num = inclusion_proof.location().block_num();
    let (header, _) = node.get_block_header_by_number(Some(block_num), false).await.unwrap();
    let state = CommittedNoteState {
        inclusion_proof,
        metadata: *public_note.metadata(),
        block_note_root: header.note_root(),
    };
    InputNoteRecord::new(
        NoteDetails::from(public_note.clone()),
        public_note.attachments().clone(),
        None,
        state.into(),
    )
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

/// A sync whose chain fetch spans a sync of another client fetches the chain again and succeeds.
#[tokio::test]
async fn sync_chain_fetches_again_after_another_sync_moves_the_store() {
    let (node, _notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let rpc = GatedRpc::new(node.clone());
    let store_path = create_test_store_path();
    let mut syncer = client_on(&store_path, rpc.clone(), false).await;
    let mut other = client_on(&store_path, node.clone(), false).await;

    syncer.sync_chain().await.unwrap();
    node.advance_blocks(10);

    rpc.chain_gate.arm();
    let requests_before = rpc.chain_requests();
    let (sync, other_sync) =
        Box::pin(while_held(&rpc.chain_gate, syncer.sync_chain(), other.sync_chain())).await;
    other_sync
        .expect("the sync must fetch the chain")
        .expect("the other sync must succeed");

    sync.expect("the sync must fetch the chain again and succeed");
    assert_eq!(rpc.chain_requests(), requests_before + 2);
    assert_eq!(syncer.get_sync_height().await.unwrap(), node.get_chain_tip_block_num());
}

/// A full sync whose chain fetch spans a sync of another client fetches the chain again and
/// succeeds.
#[tokio::test]
async fn sync_state_fetches_again_after_another_sync_moves_the_store() {
    let (node, _notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let rpc = GatedRpc::new(node.clone());
    let store_path = create_test_store_path();
    let mut syncer = client_on(&store_path, rpc.clone(), false).await;
    let mut other = client_on(&store_path, node.clone(), false).await;

    syncer.sync_state().await.unwrap();
    node.advance_blocks(10);

    rpc.chain_gate.arm();
    let requests_before = rpc.chain_requests();
    let (sync, other_sync) =
        Box::pin(while_held(&rpc.chain_gate, syncer.sync_state(), other.sync_chain())).await;
    other_sync
        .expect("the sync must fetch the chain")
        .expect("the other sync must succeed");

    sync.expect("the sync must fetch the chain again and succeed");
    assert_eq!(rpc.chain_requests(), requests_before + 2);
    assert_eq!(syncer.get_sync_height().await.unwrap(), node.get_chain_tip_block_num());
}

/// An import by details whose note block insert the store refuses, because another client synced
/// across a tree merge after the import read the partial MMR, stores the full path of every note
/// block.
#[tokio::test]
async fn import_by_details_with_a_refused_note_block_stores_every_full_path() {
    let note_blocks = [NOTE_BLOCK, 6];
    let (node, notes) = node_with_notes(&note_blocks).await;
    node.advance_blocks(4);
    let rpc = GatedRpc::new(node.clone());
    let (mut importer, store, store_path) = refusing_client(rpc, |builder| builder).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;

    importer.sync_state().await.unwrap();
    let height_before = importer.get_sync_height().await.unwrap();

    store.hold_insert_of(BlockNumber::from(NOTE_BLOCK));
    let note_files: Vec<NoteFile> = notes
        .iter()
        .map(|note| NoteFile::ExpectedNote {
            details: note.clone().into(),
            sync_hint: NoteSyncHint::new(BlockNumber::GENESIS, note.metadata().tag()),
        })
        .collect();
    let (import, height_after) =
        Box::pin(while_held(&store.insert_gate, importer.import_notes(&note_files), async {
            node.advance_blocks(40);
            syncer.sync_state().await.unwrap();
            syncer.get_sync_height().await.unwrap()
        }))
        .await;
    let height_after = height_after.expect("the import must store the note block");
    assert!(path_depth(height_after, NOTE_BLOCK) > path_depth(height_before, NOTE_BLOCK));

    import.expect("the import must succeed");
    for block in note_blocks {
        assert_full_path(&store_path, &node, block, height_after).await;
    }
}

/// An import by proof whose first proof is for a smaller forest than the partial MMR requests the
/// proof again and stores the full path.
#[tokio::test]
async fn import_by_proof_with_a_lagging_proof_requests_it_again() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    node.advance_blocks(HEAL_HEIGHT - node.get_chain_tip_block_num().as_u32());
    let rpc = GatedRpc::new(node.clone());
    let store_path = create_test_store_path();
    let mut importer = client_on(&store_path, rpc.clone(), false).await;
    importer.sync_state().await.unwrap();
    let height = importer.get_sync_height().await.unwrap();

    *rpc.next_proof_forest.lock().unwrap() = Some(Forest::new(40).unwrap());
    importer
        .import_notes(&[NoteFile::NoteId(notes[0].id())])
        .await
        .expect("the import must request the proof again and succeed");

    assert_eq!(node.block_header_requests(NOTE_BLOCK.into()), [true, true]);
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;
}

/// An import by proof that read the partial MMR before another client synced past the block of the
/// note reads the partial MMR again and uses the proof that it holds.
#[tokio::test]
async fn import_by_proof_after_a_sync_reads_the_partial_mmr_again() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;
    importer.sync_state().await.unwrap();
    let late_block = late_note.commit(&node);

    store.read_gate.arm();
    let note_files = [NoteFile::NoteId(late_note.note.id())];
    let (import, height) =
        Box::pin(while_held(&store.read_gate, importer.import_notes(&note_files), async {
            syncer.sync_chain().await.unwrap();
            syncer.get_sync_height().await.unwrap()
        }))
        .await;
    let height = height.expect("the import must read the partial MMR");
    assert_eq!(height, late_block);

    import.expect("the import must read the partial MMR again and succeed");
    assert_eq!(
        node.block_header_requests(late_block),
        [true],
        "the import must use the proof that it holds"
    );
    assert_full_path(&store_path, &node, late_block.as_u32(), height).await;
}

/// A client that caches its partial MMR reads it from the store once while the store does not
/// change. The partial MMR tracks a block, so the cache check compares the tracked blocks.
#[tokio::test]
async fn cached_partial_mmr_is_read_from_the_store_once() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let (mut client, store, _) =
        refusing_client(node.clone(), |builder| builder.cache_partial_mmr_in_memory(true)).await;
    client.sync_chain().await.unwrap();
    Box::pin(client.import_notes(&[NoteFile::NoteId(notes[0].id())])).await.unwrap();

    let reads_before = store.partial_mmr_reads();
    let first = client.get_current_partial_mmr().await.unwrap();
    let second = client.get_current_partial_mmr().await.unwrap();

    assert!(
        first.is_tracked(NOTE_BLOCK as usize),
        "the partial MMR must track the note block"
    );
    assert_eq!(first, second);
    assert_eq!(
        store.partial_mmr_reads(),
        reads_before,
        "a read with no store change must use the cached partial MMR"
    );
}

/// A prune that another client's import runs into keeps the block of the imported note: the import
/// commits a note in a block that the prune read as irrelevant, before the prune writes.
#[tokio::test]
async fn prune_keeps_a_block_that_an_import_makes_relevant() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let (mut pruner, store, store_path) =
        refusing_client(node.clone(), |builder| builder.irrelevant_block_prune_interval(Some(1)))
            .await;
    let mut importer = client_on(&store_path, node.clone(), false).await;
    pruner.sync_chain().await.unwrap();
    let height = pruner.get_sync_height().await.unwrap();
    track_with_full_path(&store_path, &node, NOTE_BLOCK, height).await;
    node.advance_blocks(1);

    store.prune_gate.arm();
    let note_files = [NoteFile::NoteId(notes[0].id())];
    let (sync, import) = Box::pin(while_held(
        &store.prune_gate,
        pruner.sync_chain(),
        importer.import_notes(&note_files),
    ))
    .await;
    import.expect("the sync must prune").expect("the import must succeed");
    sync.expect("a refused prune must not fail the sync");

    let height = pruner.get_sync_height().await.unwrap();
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;
    let record = importer.get_input_note(notes[0].id()).await.unwrap().unwrap();
    assert!(
        record.is_committed(),
        "the import must commit the note, got {:?}",
        record.state()
    );
}

/// An import whose note record the store refuses, because another writer removed the path of the
/// note block after the import stored it, stores the block again and writes the record again.
#[tokio::test]
async fn import_writes_a_note_again_after_the_store_refuses_its_block() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    node.advance_blocks(10);
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    importer.sync_state().await.unwrap();
    let height = importer.get_sync_height().await.unwrap();

    store.upsert_gate.arm();
    let upserts_before = store.upserts.calls();
    let note_files = [NoteFile::NoteId(notes[0].id())];
    let (import, removed) =
        Box::pin(while_held(&store.upsert_gate, importer.import_notes(&note_files), async {
            untrack_with_sql(&store_path, NOTE_BLOCK);
            delete_nodes(&store_path, &leaf_and_path_indices(NOTE_BLOCK, height));
        }))
        .await;
    removed.expect("the import must write the note record");

    import.expect("the import must store the block again and succeed");
    assert_eq!(
        store.upserts.calls(),
        upserts_before + 2,
        "the store must refuse the first write of the record"
    );
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;
    let record = importer.get_input_note(notes[0].id()).await.unwrap().unwrap();
    assert!(
        record.is_committed(),
        "the import must commit the note, got {:?}",
        record.state()
    );
}

/// An import by proof whose partial MMR tracks the note block, which another client prunes after
/// the import read the partial MMR, reads the partial MMR again and stores the block.
#[tokio::test]
async fn import_by_proof_stores_a_block_pruned_after_its_read() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    node.advance_blocks(10);
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    importer.sync_state().await.unwrap();
    let height = importer.get_sync_height().await.unwrap();
    track_with_full_path(&store_path, &node, NOTE_BLOCK, height).await;

    store.read_gate.arm();
    let note_files = [NoteFile::NoteId(notes[0].id())];
    let (import, pruned) =
        Box::pin(while_held(&store.read_gate, importer.import_notes(&note_files), async {
            untrack_with_sql(&store_path, NOTE_BLOCK);
            delete_nodes(&store_path, &leaf_and_path_indices(NOTE_BLOCK, height));
            delete_header_with_sql(&store_path, NOTE_BLOCK);
        }))
        .await;
    pruned.expect("the import must read the partial MMR");

    import.expect("the import must read the partial MMR again and succeed");
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;
    let record = importer.get_input_note(notes[0].id()).await.unwrap().unwrap();
    assert!(
        record.is_committed(),
        "the import must commit the note, got {:?}",
        record.state()
    );
}

/// An import whose note record waits while another client syncs and prunes the note block, which
/// deletes the header of the block, stores the block again with its full path and commits the note.
#[tokio::test]
async fn import_stores_a_note_block_that_a_prune_deletes_before_the_note_write() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    node.advance_blocks(10);
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    let mut pruner = build_client(
        ClientBuilder::new()
            .rpc(node.clone())
            .sqlite_store(store_path.clone())
            .irrelevant_block_prune_interval(Some(1)),
    )
    .await;
    importer.sync_state().await.unwrap();
    node.advance_blocks(1);

    store.upsert_gate.arm();
    let upserts_before = store.upserts.calls();
    let note_files = [NoteFile::NoteId(notes[0].id())];
    let (import, header_after_prune) =
        Box::pin(while_held(&store.upsert_gate, importer.import_notes(&note_files), async {
            pruner.sync_chain().await.unwrap();
            SqliteStore::new(store_path.clone())
                .await
                .unwrap()
                .get_block_header_by_num(BlockNumber::from(NOTE_BLOCK))
                .await
                .unwrap()
        }))
        .await;
    assert!(
        header_after_prune.expect("the import must write the note record").is_none(),
        "the sync must prune the header of the note block"
    );

    import.expect("the import must store the block again and succeed");
    let height = importer.get_sync_height().await.unwrap();
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;
    let record = importer.get_input_note(notes[0].id()).await.unwrap().unwrap();
    assert!(
        record.is_committed(),
        "the import must commit the note, got {:?}",
        record.state()
    );
    assert_eq!(
        store.upserts.calls(),
        upserts_before + 2,
        "the store must refuse the first write of the record"
    );
}

/// An import by proof of a note in a block after the sync height, whose block another client syncs
/// to before the import writes the record, succeeds and stores the full path of that block.
#[tokio::test]
async fn import_by_proof_of_a_future_note_survives_a_sync_to_its_block() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;
    importer.sync_state().await.unwrap();
    let late_block = late_note.commit(&node);

    store.upsert_gate.arm();
    let note_files = [NoteFile::NoteId(late_note.note.id())];
    let (import, height) =
        Box::pin(while_held(&store.upsert_gate, importer.import_notes(&note_files), async {
            syncer.sync_chain().await.unwrap();
            syncer.get_sync_height().await.unwrap()
        }))
        .await;
    assert_eq!(height.expect("the import must write the note record"), late_block);

    import.expect("the import must succeed");
    let record = importer
        .get_input_note(late_note.note.id())
        .await
        .unwrap()
        .expect("the note must be stored");
    assert!(
        record.is_committed(),
        "the import must verify the note against its block, got {:?}",
        record.state()
    );
    assert_full_path(&store_path, &node, late_block.as_u32(), late_block).await;
}

/// A PSWAP round note, which the sync writes before it stores the headers of its blocks, is
/// accepted when the store does not have the header of its block.
#[tokio::test]
async fn round_note_of_a_block_without_a_stored_header_is_accepted() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let store_path = create_test_store_path();
    let mut client = client_on(&store_path, node.clone(), false).await;
    client.sync_state().await.unwrap();
    let late_block = late_note.commit(&node);

    let note_id = late_note.note.id();
    let Some(FetchedNote::Public(round_note, inclusion_proof)) =
        node.get_notes_by_id(&[note_id]).await.unwrap().pop()
    else {
        panic!("the node must return the public note");
    };
    let (header, _) = node.get_block_header_by_number(Some(late_block), false).await.unwrap();
    let state = CommittedNoteState {
        inclusion_proof,
        metadata: *round_note.metadata(),
        block_note_root: header.note_root(),
    };
    let record = InputNoteRecord::new(
        NoteDetails::from(round_note.clone()),
        round_note.attachments().clone(),
        None,
        state.into(),
    );

    let store = SqliteStore::new(store_path.clone()).await.unwrap();
    assert!(store.get_block_header_by_num(late_block).await.unwrap().is_none());
    store
        .upsert_input_notes(&[record])
        .await
        .expect("the store must accept the note");

    assert_eq!(store.get_input_notes(NoteFilter::List(vec![note_id])).await.unwrap().len(), 1);
    raw_partial_mmr(&store_path).await.expect("the store must stay readable");
}

// REFUSED WRITE TESTS
// ================================================================================================

/// A sync whose write the store refuses once fetches the chain again and succeeds.
#[tokio::test]
async fn sync_chain_retries_a_refused_write() {
    let (node, _notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let (mut client, store, _) = refusing_client(node.clone(), |builder| builder).await;

    store.syncs.refuse_next(1);
    client
        .sync_chain()
        .await
        .expect("the sync must fetch the chain again and succeed");

    assert_eq!(store.syncs.calls(), 2);
    assert_eq!(client.get_sync_height().await.unwrap(), node.get_chain_tip_block_num());
}

/// A sync whose write the store refuses on every attempt returns the error of a stale chain write.
#[tokio::test]
async fn sync_chain_stops_after_the_last_refused_write() {
    let (node, _notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let (mut client, store, _) = refusing_client(node.clone(), |builder| builder).await;

    store.syncs.refuse_next(STALE_WRITE_ATTEMPTS);
    let result = client.sync_chain().await;

    assert!(is_stale_write(&result), "the sync must return the stale write, got {result:?}");
    assert_eq!(store.syncs.calls(), STALE_WRITE_ATTEMPTS);
    assert_eq!(client.get_sync_height().await.unwrap(), BlockNumber::GENESIS);
}

/// A full sync whose write the store refuses once fetches the chain again, and it commits the note
/// that it received from the note transport.
#[tokio::test]
async fn sync_state_retries_a_refused_write_and_commits_a_transport_note() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let transport_note = &notes[0];
    let transport = Arc::new(RwLock::new(MockNoteTransportNode::new()));
    transport
        .write()
        .add_note(*transport_note.header(), NoteDetails::from(transport_note.clone()).to_bytes());
    let (mut client, store, _) = refusing_client(node.clone(), |builder| {
        builder.note_transport(Arc::new(MockNoteTransportApi::new(transport.clone())))
    })
    .await;
    client.add_note_tag(transport_note.metadata().tag()).await.unwrap();

    store.syncs.refuse_next(1);
    let summary = client.sync_state().await.expect("the sync must fetch the chain again");

    assert_eq!(store.syncs.calls(), 2);
    assert_eq!(summary.new_private_notes, [transport_note.id()]);
    let record = client
        .get_input_note(transport_note.id())
        .await
        .unwrap()
        .expect("the note must be stored");
    assert!(record.is_committed(), "the sync must commit the note, got {:?}", record.state());
}

/// A full sync whose PSWAP round note write the store refuses once fetches the chain again and
/// stores the round note. The order is private, so only the round applies the payback note.
#[tokio::test]
async fn sync_state_fetches_again_after_a_refused_pswap_round_note_write() {
    let node = MockRpcApi::new(Box::pin(create_prebuilt_mock_chain()).await);
    let (creator, store, _) = refusing_client(Arc::new(node.clone()), |builder| builder).await;
    let mut creator = TestClient::from(creator);
    let mut filler = create_pswap_test_client(&node).await;

    let (creator_wallet, offered_faucet) =
        creator.setup_wallet_and_faucet(AccountType::Private).await.unwrap();
    let (filler_wallet, requested_faucet) =
        filler.setup_wallet_and_faucet(AccountType::Private).await.unwrap();
    creator
        .mint_and_consume(creator_wallet.id(), offered_faucet.id(), NoteType::Private)
        .await
        .unwrap();
    node.prove_block();
    creator.sync_state().await.unwrap();
    filler
        .mint_and_consume(filler_wallet.id(), requested_faucet.id(), NoteType::Private)
        .await
        .unwrap();
    node.prove_block();
    filler.sync_state().await.unwrap();

    let pswap_data = PswapTransactionData::new(
        creator_wallet.id(),
        FungibleAsset::new(offered_faucet.id(), 100).unwrap(),
        FungibleAsset::new(requested_faucet.id(), 50).unwrap(),
    );
    let create_request = TransactionRequestBuilder::new()
        .build_pswap_create(&pswap_data, NoteType::Private, NoteType::Private, None, creator.rng())
        .unwrap();
    let pswap_note = create_request.expected_output_own_notes()[0].clone();
    let pswap = PswapNote::try_from(&pswap_note).unwrap();
    Box::pin(creator.submit_new_transaction(creator_wallet.id(), create_request))
        .await
        .unwrap();
    node.prove_block();
    creator.sync_state().await.unwrap();

    let fill_amount = AssetAmount::new(25).unwrap();
    let fill_request = TransactionRequestBuilder::new()
        .build_pswap_consume(&pswap_note, filler_wallet.id(), fill_amount, AssetAmount::ZERO)
        .unwrap();
    Box::pin(filler.submit_new_transaction(filler_wallet.id(), fill_request))
        .await
        .unwrap();
    node.prove_block();
    filler.sync_state().await.unwrap();

    store.upserts.refuse_next(1);
    let upserts_before = store.upserts.calls();
    creator
        .sync_state()
        .await
        .expect("the sync must fetch the chain again and succeed");

    let lineage = creator.pswap_lineage(pswap.order_id()).await.unwrap().unwrap();
    assert_eq!(lineage.current_depth, 1, "the sync must apply the round");
    let payback_attachment = PswapNoteAttachment::new(fill_amount, pswap.order_id(), 1);
    let payback_id = pswap.payback_note(filler_wallet.id(), &payback_attachment).unwrap().id();
    let payback = creator
        .get_input_note(payback_id)
        .await
        .unwrap()
        .expect("the sync must store the payback note");
    assert!(
        payback.is_committed(),
        "the payback must be committed, got {:?}",
        payback.state()
    );
    assert_eq!(
        store.upserts.calls(),
        upserts_before + 3,
        "the refused payback write, then the payback and the remainder writes"
    );
}

/// A full sync whose write the store refuses on every attempt returns the error of a stale chain
/// write.
#[tokio::test]
async fn sync_state_stops_after_the_last_refused_write() {
    let (node, _notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let (mut client, store, _) = refusing_client(node.clone(), |builder| builder).await;

    store.syncs.refuse_next(STALE_WRITE_ATTEMPTS);
    let result = client.sync_state().await;

    assert!(is_stale_write(&result), "the sync must return the stale write, got {result:?}");
    assert_eq!(store.syncs.calls(), STALE_WRITE_ATTEMPTS);
    assert_eq!(client.get_sync_height().await.unwrap(), BlockNumber::GENESIS);
}

/// A prune that the store refuses does not fail the sync and does not record the prune height, so
/// the next sync prunes again. The refused prune leaves the store readable.
#[tokio::test]
async fn refused_prune_runs_again_on_the_next_sync() {
    let (node, _notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let (mut client, store, store_path) =
        refusing_client(node.clone(), |builder| builder.irrelevant_block_prune_interval(Some(100)))
            .await;

    store.prunes.refuse_next(1);
    client.sync_chain().await.expect("a refused prune must not fail the sync");
    assert_eq!(store.prunes.calls(), 1, "the client must not prune again in the same sync");
    raw_partial_mmr(&store_path).await.expect("the store must stay readable");

    client.sync_chain().await.unwrap();
    assert_eq!(store.prunes.calls(), 2, "the next sync must prune again");
}

/// A sync update that the store refuses returns the error of a stale chain write. The client does
/// not prune, and it drops its cached partial MMR.
#[tokio::test]
async fn refused_apply_state_sync_returns_the_stale_write() {
    let (node, _notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let (mut client, store, _) =
        refusing_client(node.clone(), |builder| builder.cache_partial_mmr_in_memory(true)).await;
    client.sync_chain().await.unwrap();
    assert!(client.test_has_cached_partial_mmr());
    let height = client.get_sync_height().await.unwrap();
    node.advance_blocks(5);

    let state_sync = StateSync::new(
        node.clone(),
        Arc::new(client.note_screener()),
        None,
        client.get_validator_config().await.unwrap(),
    );
    let mut partial_mmr = client.get_current_partial_mmr().await.unwrap();
    let input = client.build_sync_input().await.unwrap();
    let update = Box::pin(state_sync.sync_state(&mut partial_mmr, input)).await.unwrap();

    store.syncs.refuse_next(1);
    let prunes_before = store.prunes.calls();
    let result = client.apply_state_sync(update).await;

    assert!(is_stale_write(&result), "the update must be refused, got {result:?}");
    assert_eq!(client.get_sync_height().await.unwrap(), height);
    assert_eq!(store.prunes.calls(), prunes_before, "a refused update must not prune");
    assert!(
        !client.test_has_cached_partial_mmr(),
        "a refused update must drop the cached MMR"
    );
}

/// An import by details whose first two note block writes the store refuses, after another client
/// synced across a tree merge, writes the block a third time and stores every full path. A proof
/// that the import requests again does not use a write.
#[tokio::test]
async fn import_by_details_counts_only_writes_against_the_stale_write_budget() {
    let note_blocks = [NOTE_BLOCK, 6];
    let (node, notes) = node_with_notes(&note_blocks).await;
    node.advance_blocks(4);
    let rpc = GatedRpc::new(node.clone());
    let (mut importer, store, store_path) = refusing_client(rpc, |builder| builder).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;
    importer.sync_state().await.unwrap();

    store.hold_insert_of(BlockNumber::from(NOTE_BLOCK));
    let note_files: Vec<NoteFile> = notes
        .iter()
        .map(|note| NoteFile::ExpectedNote {
            details: note.clone().into(),
            sync_hint: NoteSyncHint::new(BlockNumber::GENESIS, note.metadata().tag()),
        })
        .collect();
    let (import, held) =
        Box::pin(while_held(&store.insert_gate, importer.import_notes(&note_files), async {
            node.advance_blocks(40);
            syncer.sync_state().await.unwrap();
            // The held insert counts its call after the gate, so the refusals start with it.
            store.inserts.refuse_next(STALE_WRITE_ATTEMPTS - 1);
            (syncer.get_sync_height().await.unwrap(), store.inserts.calls())
        }))
        .await;
    let (height, inserts_before) = held.expect("the import must store the note block");

    import.expect("the import must write the note block a third time and succeed");
    assert_eq!(
        store.inserts.calls(),
        inserts_before + STALE_WRITE_ATTEMPTS + 1,
        "the import must write the first note block {STALE_WRITE_ATTEMPTS} times and the second once"
    );
    for block in note_blocks {
        assert_full_path(&store_path, &node, block, height).await;
    }
}

/// An import by proof that reads the partial MMR again because it is older than the note block, and
/// whose writes the store then refuses, returns the error of a stale chain write after the last
/// write.
#[tokio::test]
async fn import_by_proof_returns_the_stale_write_when_its_writes_run_out() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;
    let mut later_syncer = client_on(&store_path, node.clone(), false).await;
    importer.sync_state().await.unwrap();
    let late_block = late_note.commit(&node);

    store.read_gate.arm();
    store.hold_insert_of(late_block);
    let note_files = [NoteFile::NoteId(late_note.note.id())];
    let import = while_held(&store.insert_gate, importer.import_notes(&note_files), async {
        node.advance_blocks(40);
        later_syncer.sync_chain().await.unwrap();
        // The held insert counts its call after the gate, so the refusals start with it.
        store.inserts.refuse_next(STALE_WRITE_ATTEMPTS);
        store.inserts.calls()
    });
    let ((import, inserts_before), sync_ran) =
        Box::pin(while_held(&store.read_gate, import, async {
            syncer.sync_chain().await.unwrap();
        }))
        .await;
    sync_ran.expect("the import must read the partial MMR");
    let inserts_before = inserts_before.expect("the import must store the note block");

    assert!(
        is_stale_write(&import),
        "the import must return the stale write, got {import:?}"
    );
    assert_eq!(store.inserts.calls(), inserts_before + STALE_WRITE_ATTEMPTS);
    assert_eq!(
        node.block_header_requests(late_block),
        [true, true],
        "the import must request the proof again once, after the forest grew"
    );
}

/// An import by proof from a node whose proofs stay at a smaller forest than the store requests the
/// proof once more, then returns an error that the node is behind and writes nothing.
#[tokio::test]
async fn import_by_proof_from_a_node_behind_the_store_writes_nothing() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    node.advance_blocks(HEAL_HEIGHT - node.get_chain_tip_block_num().as_u32());
    let rpc = GatedRpc::new(node.clone());
    let store_path = create_test_store_path();
    let mut importer = client_on(&store_path, rpc.clone(), false).await;
    importer.sync_state().await.unwrap();
    let nodes_before = stored_nodes(&store_path).await;

    *rpc.proof_forest.lock().unwrap() = Some(Forest::new(40).unwrap());
    let requests_before = rpc.proof_requests();
    let result = importer.import_notes(&[NoteFile::NoteId(notes[0].id())]).await;

    assert!(
        matches!(&result, Err(ClientError::ChainValidationError(message)) if message.contains("behind")),
        "the import must return that the node is behind, got {result:?}"
    );
    assert_eq!(rpc.proof_requests(), requests_before + 2);
    let header = importer.get_block_header_by_num(NOTE_BLOCK.into()).await.unwrap();
    assert!(header.is_none(), "the import must not store the note block");
    assert_eq!(stored_nodes(&store_path).await, nodes_before);
}

// NOTE BLOCK HEADER TESTS
// ================================================================================================

/// An import by details of a note in a block that the client tracks refuses a note sync whose
/// header for that block differs from the stored header, and keeps the record expected.
#[tokio::test]
async fn import_by_details_refuses_a_header_that_differs_from_the_tracked_header() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let rpc = GatedRpc::new(node.clone());
    let store_path = create_test_store_path();
    let mut importer = client_on(&store_path, rpc.clone(), false).await;
    importer.sync_state().await.unwrap();
    let height = importer.get_sync_height().await.unwrap();

    let details: NoteDetails = notes[0].clone().into();
    let tag = notes[0].metadata().tag();
    importer
        .import_notes(&[NoteFile::ExpectedNote {
            details: details.clone(),
            sync_hint: NoteSyncHint::new(height + 1, tag),
        }])
        .await
        .unwrap();
    track_with_full_path(&store_path, &node, NOTE_BLOCK, height).await;

    rpc.forge_note_blocks.store(true, Ordering::SeqCst);
    let result = importer
        .import_notes(&[NoteFile::ExpectedNote {
            details: details.clone(),
            sync_hint: NoteSyncHint::new(BlockNumber::GENESIS, tag),
        }])
        .await;

    let records = importer
        .get_input_notes(NoteFilter::DetailsCommitments(vec![details.commitment()]))
        .await
        .unwrap();
    assert!(
        matches!(records[0].state(), InputNoteState::Expected(_)),
        "the record must stay expected, got {:?}",
        records[0].state()
    );
    assert!(
        matches!(result, Err(ClientError::ChainValidationError(_))),
        "the import must refuse the header, got {result:?}"
    );
}

/// An import by details whose note block write the store refuses requests the proof again, and
/// refuses a header from the node that differs from the header of the note sync.
#[tokio::test]
async fn import_by_details_refuses_a_requested_header_that_differs_from_the_note_sync() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let rpc = GatedRpc::new(node.clone());
    let store_path = create_test_store_path();
    let mut importer = client_on(&store_path, rpc.clone(), false).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;
    importer.sync_state().await.unwrap();

    rpc.forge_proofs.store(true, Ordering::SeqCst);
    rpc.sync_notes_gate.arm();
    let note_files = [NoteFile::ExpectedNote {
        details: notes[0].clone().into(),
        sync_hint: NoteSyncHint::new(BlockNumber::GENESIS, notes[0].metadata().tag()),
    }];
    let (import, sync_ran) =
        Box::pin(while_held(&rpc.sync_notes_gate, importer.import_notes(&note_files), async {
            node.advance_blocks(40);
            syncer.sync_state().await.unwrap();
        }))
        .await;
    sync_ran.expect("the import must sync the notes");

    assert!(
        matches!(&import, Err(ClientError::ChainValidationError(message))
            if message.contains("two different headers")),
        "the import must refuse the requested header, got {import:?}"
    );
    assert_eq!(rpc.proof_requests(), 1);
}

// NOTE RE-IMPORT TESTS
// ================================================================================================

/// A re-import by note ID of a stored expected note in a block at or below the sync height stores
/// and authenticates the block before the write, so the store accepts the first write of the
/// committed record.
#[tokio::test]
async fn import_by_id_of_a_stored_expected_note_writes_it_committed_once() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    node.advance_blocks(10);
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    importer.sync_state().await.unwrap();
    let height = importer.get_sync_height().await.unwrap();
    let tag = store_expected_note(&store, &notes[0]).await;

    let upserts_before = store.upserts.calls();
    importer.import_notes(&[NoteFile::NoteId(notes[0].id())]).await.unwrap();

    let record = importer.get_input_note(notes[0].id()).await.unwrap().unwrap();
    assert!(
        record.is_committed(),
        "the import must commit the note, got {:?}",
        record.state()
    );
    assert_eq!(
        store.upserts.calls(),
        upserts_before + 1,
        "the store must accept the first write of the record"
    );
    assert_full_path(&store_path, &node, NOTE_BLOCK, height).await;
    assert!(
        !has_note_tag(&importer, tag).await,
        "the import must remove the note tag of the committed note"
    );
}

/// A re-import by note ID of a stored expected note that is committed and spent at or below the
/// sync height writes the note consumed and removes its note tag.
#[tokio::test]
async fn import_by_id_of_a_stored_expected_note_spent_at_or_below_the_sync_height_consumes_it() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    let spend_block = spend_note(&node, &notes[0]).await;
    node.advance_blocks(10);
    let (mut importer, store, _) = refusing_client(node.clone(), |builder| builder).await;
    Box::pin(importer.sync_state()).await.unwrap();
    let tag = store_expected_note(&store, &notes[0]).await;

    Box::pin(importer.import_notes(&[NoteFile::NoteId(notes[0].id())]))
        .await
        .unwrap();

    let record = importer.get_input_note(notes[0].id()).await.unwrap().unwrap();
    assert!(
        matches!(record.state(), InputNoteState::ConsumedExternal(_)),
        "the import must write the spent note consumed, got {:?}",
        record.state()
    );
    assert_eq!(record.state().consumed_block_height(), Some(spend_block));
    assert!(
        !has_note_tag(&importer, tag).await,
        "the import must remove the note tag of the consumed note"
    );
}

/// A re-import by note ID of a stored expected note in a block after the sync height, whose note
/// tag the store does not have, stores the note unverified and adds its note tag. The next sync of
/// the client finds the note through that tag and verifies it.
#[tokio::test]
async fn import_by_id_of_a_stored_expected_note_after_the_sync_height_adds_its_tag() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let (mut importer, store, _) = refusing_client(node.clone(), |builder| builder).await;
    importer.sync_state().await.unwrap();
    let tag = store_expected_note(&store, &late_note.note).await;
    assert_eq!(store.remove_note_tag(tag).await.unwrap(), 1);
    let late_block = late_note.commit(&node);

    Box::pin(importer.import_notes(&[NoteFile::NoteId(late_note.note.id())]))
        .await
        .unwrap();

    let record = importer.get_input_note(late_note.note.id()).await.unwrap().unwrap();
    assert!(
        matches!(record.state(), InputNoteState::Unverified(_)),
        "the import must keep the note unverified, got {:?}",
        record.state()
    );
    assert!(
        has_note_tag(&importer, tag).await,
        "the import must add the note tag of the unverified note"
    );

    Box::pin(importer.sync_state()).await.unwrap();
    assert_eq!(importer.get_sync_height().await.unwrap(), late_block);
    let record = importer.get_input_note(late_note.note.id()).await.unwrap().unwrap();
    assert!(
        record.is_committed(),
        "the next sync must verify the note, got {:?}",
        record.state()
    );
}

/// A re-import by note ID of a stored expected note in a block after the sync height, whose block
/// and spend another client syncs past while the import holds its note write, keeps the consumed
/// record that the sync writes.
#[tokio::test]
async fn import_by_id_keeps_the_record_of_a_note_that_a_sync_consumes_during_the_import() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;
    Box::pin(importer.sync_state()).await.unwrap();
    let tag = store_expected_note(&store, &late_note.note).await;
    late_note.commit(&node);
    let spend_block = spend_note(&node, &late_note.note).await;

    store.upsert_gate.arm();
    let upserts_before = store.upserts.calls();
    let note_files = [NoteFile::NoteId(late_note.note.id())];
    let (import, height) =
        Box::pin(while_held(&store.upsert_gate, importer.import_notes(&note_files), async {
            syncer.sync_chain().await.unwrap();
            syncer.get_sync_height().await.unwrap()
        }))
        .await;
    assert_eq!(height.expect("the import must write the note record"), spend_block);

    import.expect("the import must succeed");
    let record = importer.get_input_note(late_note.note.id()).await.unwrap().unwrap();
    assert!(
        matches!(record.state(), InputNoteState::ConsumedExternal(_)),
        "the import must keep the consumed record, got {:?}",
        record.state()
    );
    assert_eq!(record.state().consumed_block_height(), Some(spend_block));
    assert_eq!(
        store.upserts.calls(),
        upserts_before + 1,
        "the import must not write the note again after the sync consumes it"
    );
    assert!(
        !has_note_tag(&importer, tag).await,
        "the import must remove the note tag of the consumed note"
    );
}

/// A re-import by note ID of a stored expected note in a block after the sync height, whose block
/// and spend a sync of another client moves the sync height past while the import holds its note
/// write, writes the note consumed. The sync fetched its updates before the note was stored, so the
/// sync does not change the note.
#[tokio::test]
async fn import_by_id_of_a_note_spent_before_a_sync_past_its_block_consumes_it() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;
    Box::pin(importer.sync_state()).await.unwrap();
    late_note.commit(&node);
    let spend_block = spend_note(&node, &late_note.note).await;
    let state_sync = gated_state_sync(&syncer, node.clone(), Arc::new(Gate::default())).await;
    let mut chain_sync_data = Box::pin(syncer.fetch_chain_updates(&state_sync)).await.unwrap();
    state_sync.derive_state_updates(&mut chain_sync_data).await.unwrap();
    state_sync.fetch_nullifiers(&mut chain_sync_data).await.unwrap();
    let tag = store_expected_note(&store, &late_note.note).await;

    store.upsert_gate.arm();
    let upserts_before = store.upserts.calls();
    let note_files = [NoteFile::NoteId(late_note.note.id())];
    let (import, apply) = Box::pin(while_held(
        &store.upsert_gate,
        importer.import_notes(&note_files),
        syncer.apply_chain_updates(&state_sync, chain_sync_data),
    ))
    .await;
    apply
        .expect("the import must write the note record")
        .expect("the sync must succeed");
    assert_eq!(importer.get_sync_height().await.unwrap(), spend_block);

    import.expect("the import must succeed");
    let record = importer.get_input_note(late_note.note.id()).await.unwrap().unwrap();
    assert!(
        matches!(record.state(), InputNoteState::ConsumedExternal(_)),
        "the import must write the spent note consumed, got {:?}",
        record.state()
    );
    assert_eq!(record.state().consumed_block_height(), Some(spend_block));
    assert_eq!(
        store.upserts.calls(),
        upserts_before + 2,
        "the store must refuse the first write of the record"
    );
    assert!(
        !has_note_tag(&importer, tag).await,
        "the import must remove the note tag of the consumed note"
    );
}

/// A re-import by note ID of a stored expected note in a block at or below the sync height, whose
/// committed record waits to be written while a sync of another client moves past the spend of the
/// note and prunes its block, ends consumed. The store refuses the committed record, because the
/// header of its block is gone, and the import does not write the note committed over the spend.
#[tokio::test]
async fn import_by_id_keeps_a_spend_that_a_pruning_sync_finds_during_a_committed_write() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    node.advance_blocks(10);
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    let mut pruner = build_client(
        ClientBuilder::new()
            .rpc(node.clone())
            .sqlite_store(store_path.clone())
            .irrelevant_block_prune_interval(Some(1)),
    )
    .await;
    Box::pin(importer.sync_state()).await.unwrap();
    let tag = store_expected_note(&store, &notes[0]).await;
    // The spend is after the sync height that the import reads, so the import writes the note
    // committed.
    let spend_block = spend_note(&node, &notes[0]).await;

    store.upsert_gate.arm();
    let note_files = [NoteFile::NoteId(notes[0].id())];
    let (import, header_after_prune) =
        Box::pin(while_held(&store.upsert_gate, importer.import_notes(&note_files), async {
            pruner.sync_chain().await.unwrap();
            SqliteStore::new(store_path.clone())
                .await
                .unwrap()
                .get_block_header_by_num(BlockNumber::from(NOTE_BLOCK))
                .await
                .unwrap()
        }))
        .await;
    assert!(
        header_after_prune.expect("the import must write the note record").is_none(),
        "the sync must prune the header of the note block"
    );
    assert_eq!(importer.get_sync_height().await.unwrap(), spend_block);

    import.expect("the import must succeed");
    let record = importer.get_input_note(notes[0].id()).await.unwrap().unwrap();
    assert!(
        matches!(record.state(), InputNoteState::ConsumedExternal(_)),
        "the import must keep the spend of the note, got {:?}",
        record.state()
    );
    assert_eq!(record.state().consumed_block_height(), Some(spend_block));
    assert!(
        !has_note_tag(&importer, tag).await,
        "the import must remove the note tag of the consumed note"
    );
}

/// A re-import by note ID of a stored expected note in a block after the sync height, whose write
/// the store refuses after a sync of another client moves past the block and the spend of the note,
/// writes the note consumed over the committed record that another writer stores during the import.
/// The sync fetched its updates before the note was stored, so the sync does not change the note.
#[tokio::test]
async fn import_by_id_writes_a_spend_over_a_note_that_another_writer_commits_during_the_import() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let (mut importer, store, store_path) = refusing_client(node.clone(), |builder| builder).await;
    let mut syncer = client_on(&store_path, node.clone(), false).await;
    Box::pin(importer.sync_state()).await.unwrap();
    let late_block = late_note.commit(&node);
    let spend_block = spend_note(&node, &late_note.note).await;
    let state_sync = gated_state_sync(&syncer, node.clone(), Arc::new(Gate::default())).await;
    let mut chain_sync_data = Box::pin(syncer.fetch_chain_updates(&state_sync)).await.unwrap();
    state_sync.derive_state_updates(&mut chain_sync_data).await.unwrap();
    state_sync.fetch_nullifiers(&mut chain_sync_data).await.unwrap();
    let tag = store_expected_note(&store, &late_note.note).await;
    let committed = committed_record(&node, late_note.note.id()).await;

    store.upsert_gate.arm();
    let upserts_before = store.upserts.calls();
    let note_files = [NoteFile::NoteId(late_note.note.id())];
    let (import, other_writes) =
        Box::pin(while_held(&store.upsert_gate, importer.import_notes(&note_files), async {
            Box::pin(syncer.apply_chain_updates(&state_sync, chain_sync_data))
                .await
                .unwrap();
            // Another writer stores the note committed, and does not find its spend.
            track_with_full_path(&store_path, &node, late_block.as_u32(), spend_block).await;
            SqliteStore::new(store_path.clone())
                .await
                .unwrap()
                .upsert_input_notes(core::slice::from_ref(&committed))
                .await
                .unwrap();
        }))
        .await;
    other_writes.expect("the import must write the note record");

    import.expect("the import must succeed");
    let record = importer.get_input_note(late_note.note.id()).await.unwrap().unwrap();
    assert!(
        matches!(record.state(), InputNoteState::ConsumedExternal(_)),
        "the import must write the spend over the committed record, got {:?}",
        record.state()
    );
    assert_eq!(record.state().consumed_block_height(), Some(spend_block));
    assert_eq!(
        store.upserts.calls(),
        upserts_before + 2,
        "the store must refuse the first write of the record"
    );
    assert!(
        !has_note_tag(&importer, tag).await,
        "the import must remove the note tag of the consumed note"
    );
}

/// An import by proof of a new note in a block after the sync height stores the note unverified and
/// adds its note tag. The next sync of the client finds the note through that tag and verifies it.
#[tokio::test]
async fn import_by_proof_of_a_note_after_the_sync_height_adds_its_tag() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let (mut importer, ..) = refusing_client(node.clone(), |builder| builder).await;
    Box::pin(importer.sync_state()).await.unwrap();
    let late_block = late_note.commit(&node);
    let Some(FetchedNote::Public(note, proof)) =
        node.get_notes_by_id(&[late_note.note.id()]).await.unwrap().pop()
    else {
        panic!("the node must return the public note");
    };
    let tag = NoteTagRecord::with_note_source(note.metadata().tag(), note.details_commitment());
    assert!(!has_note_tag(&importer, tag).await);

    Box::pin(importer.import_notes(&[NoteFile::Committed { note, proof }]))
        .await
        .unwrap();

    let record = importer.get_input_note(late_note.note.id()).await.unwrap().unwrap();
    assert!(
        matches!(record.state(), InputNoteState::Unverified(_)),
        "the import must store the note unverified, got {:?}",
        record.state()
    );
    assert!(
        has_note_tag(&importer, tag).await,
        "the import must add the note tag of the unverified note"
    );

    Box::pin(importer.sync_state()).await.unwrap();
    assert_eq!(importer.get_sync_height().await.unwrap(), late_block);
    let record = importer.get_input_note(late_note.note.id()).await.unwrap().unwrap();
    assert!(
        record.is_committed(),
        "the next sync must verify the note, got {:?}",
        record.state()
    );
}

/// An import by proof of a new note in a block after the sync height, whose first write the store
/// refuses as it does when the database is locked, writes the same record again. The note stays
/// unverified with its note tag, and the import requests no block proof.
#[tokio::test]
async fn import_by_proof_writes_a_note_after_the_sync_height_again_after_a_lock_timeout() {
    let (node, _notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let rpc = GatedRpc::new(node.clone());
    let (mut importer, store, _) = refusing_client(rpc.clone(), |builder| builder).await;
    Box::pin(importer.sync_state()).await.unwrap();
    late_note.commit(&node);
    let Some(FetchedNote::Public(public_note, proof)) =
        node.get_notes_by_id(&[late_note.note.id()]).await.unwrap().pop()
    else {
        panic!("the node must return the public note");
    };
    let tag = NoteTagRecord::with_note_source(
        public_note.metadata().tag(),
        public_note.details_commitment(),
    );

    store.upserts.refuse_next(1);
    let upserts_before = store.upserts.calls();
    let requests_before = rpc.proof_requests();
    Box::pin(importer.import_notes(&[NoteFile::Committed { note: public_note, proof }]))
        .await
        .expect("the import must write the note again and succeed");

    let record = importer.get_input_note(late_note.note.id()).await.unwrap().unwrap();
    assert!(
        matches!(record.state(), InputNoteState::Unverified(_)),
        "the import must store the note unverified, got {:?}",
        record.state()
    );
    assert!(
        has_note_tag(&importer, tag).await,
        "the import must keep the note tag of the unverified note"
    );
    assert_eq!(
        rpc.proof_requests(),
        requests_before,
        "the import must not request a block proof"
    );
    assert_eq!(
        store.upserts.calls(),
        upserts_before + 2,
        "the import must write the same record a second time"
    );
}

/// A re-import by details of a stored expected note whose writes the store refuses on every attempt
/// returns the error of a stale chain write and keeps the note tag.
#[tokio::test]
async fn import_by_details_whose_writes_run_out_keeps_the_note_tag() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK]).await;
    node.advance_blocks(10);
    let (mut importer, store, _) = refusing_client(node.clone(), |builder| builder).await;
    importer.sync_state().await.unwrap();
    let tag = store_expected_note(&store, &notes[0]).await;

    store.upserts.refuse_next(STALE_WRITE_ATTEMPTS);
    let result = importer.import_notes(&[expected_note_file(&notes[0])]).await;

    assert!(
        is_stale_write(&result),
        "the import must return the stale write, got {result:?}"
    );
    assert!(
        has_note_tag(&importer, tag).await,
        "an import that fails must keep the note tag"
    );
}

/// An import of a note by ID and a note by details, whose first note write the store refuses on
/// every attempt, keeps the note tag of each note.
#[tokio::test]
async fn import_whose_first_note_write_runs_out_keeps_the_tag_of_each_note() {
    let (node, notes) = node_with_notes(&[NOTE_BLOCK, 2]).await;
    node.advance_blocks(10);
    let (mut importer, store, _) = refusing_client(node.clone(), |builder| builder).await;
    importer.sync_state().await.unwrap();
    let id_tag = store_expected_note(&store, &notes[0]).await;
    let details_tag = store_expected_note(&store, &notes[1]).await;

    store.upserts.refuse_next(STALE_WRITE_ATTEMPTS);
    let note_files = [NoteFile::NoteId(notes[0].id()), expected_note_file(&notes[1])];
    let result = importer.import_notes(&note_files).await;

    assert!(
        is_stale_write(&result),
        "the import must return the stale write, got {result:?}"
    );
    assert!(
        has_note_tag(&importer, details_tag).await,
        "the import must keep the note tag of the note that it did not write"
    );
    assert!(
        has_note_tag(&importer, id_tag).await,
        "the import must keep the note tag of the note whose write failed"
    );
}

// HEAL TESTS
// ================================================================================================

/// A store that lacks the upper siblings of a tracked block, as in the field report, is repaired
/// with one proof request.
#[tokio::test]
async fn partial_mmr_read_heals_missing_upper_siblings() {
    let HealSetup { mut client, rpc, node, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
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
    let HealSetup { client, rpc, node, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
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

/// A heal whose plan starts after another client repaired the store returns the result of a new
/// read.
#[tokio::test]
async fn partial_mmr_heal_reads_again_after_another_client_heals() {
    let HealSetup { client, node, store, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
    let other = client_on(&store_path, node.clone(), false).await;
    delete_upper_siblings(&store_path, NOTE_BLOCK);

    store.peaks_gate.arm();
    let (result, healed) = Box::pin(while_held(
        &store.peaks_gate,
        client.get_current_partial_mmr(),
        other.get_current_partial_mmr(),
    ))
    .await;
    healed
        .expect("the heal must read the stored peaks")
        .expect("the other client must repair the store");

    result.expect("the read must return the result of a new read");
    assert_full_path(&store_path, &node, NOTE_BLOCK, BlockNumber::from(HEAL_HEIGHT)).await;
}

/// A heal whose plan read the stored peaks before a sync of another client tracked a later block
/// stops before it requests a proof, and returns the result of a new read. The new read fails,
/// because block 1 is broken again when the heal continues.
#[tokio::test]
async fn partial_mmr_heal_reads_again_after_a_sync_tracks_a_later_block() {
    let (node, notes, late_note) = node_with_a_late_note(&[NOTE_BLOCK]).await;
    let HealSetup { client, rpc, store, store_path, .. } =
        heal_setup_on(node.clone(), &notes, &[NOTE_BLOCK]).await;
    let mut other = build_client(
        ClientBuilder::new()
            .rpc(node.clone())
            .sqlite_store(store_path.clone())
            .irrelevant_block_prune_interval(None),
    )
    .await;
    delete_upper_siblings(&store_path, NOTE_BLOCK);

    let late_file = NoteFile::ExpectedNote {
        details: late_note.note.clone().into(),
        sync_hint: NoteSyncHint::new(HEAL_HEIGHT.into(), late_note.note.metadata().tag()),
    };
    store.peaks_gate.arm();
    let requests_before = rpc.proof_requests();
    let reads_before = store.partial_mmr_reads();
    let (result, late_block) =
        Box::pin(while_held(&store.peaks_gate, client.get_current_partial_mmr(), async {
            // The import of the other client repairs block 1 before the sync tracks the later
            // block.
            Box::pin(other.import_notes(&[late_file])).await.unwrap();
            let late_block = late_note.commit(&node);
            other.sync_chain().await.unwrap();
            // At the new sync height, block 1 is in the same tree, so these are its upper siblings.
            delete_upper_siblings(&store_path, NOTE_BLOCK);
            late_block
        }))
        .await;
    let late_block = late_block.expect("the heal must read the stored peaks");
    let tracked = store.get_tracked_block_header_numbers().await.unwrap();
    assert!(tracked.contains(&late_block.as_usize()), "the sync must track the later block");

    assert!(
        is_missing_path_of(&result, NOTE_BLOCK),
        "the read must return the result of a new read, got {result:?}"
    );
    assert_eq!(rpc.proof_requests(), requests_before, "the heal must not request a proof");
    assert_eq!(store.partial_mmr_reads(), reads_before + 2, "the heal must read again");

    client
        .get_current_partial_mmr()
        .await
        .expect("a later read must repair the store");
    assert_full_path(&store_path, &node, NOTE_BLOCK, late_block).await;
}

/// A heal whose plan read the stored peaks before another writer deleted the header of the sync
/// height stops before it requests a proof, and returns the result of a new read.
#[tokio::test]
async fn partial_mmr_heal_reads_again_after_the_sync_height_header_is_deleted() {
    let HealSetup { client, rpc, store, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
    delete_upper_siblings(&store_path, NOTE_BLOCK);
    let nodes_before = stored_nodes(&store_path).await;

    store.peaks_gate.arm();
    let requests_before = rpc.proof_requests();
    let (result, deleted) =
        Box::pin(while_held(&store.peaks_gate, client.get_current_partial_mmr(), async {
            delete_header_with_sql(&store_path, HEAL_HEIGHT);
        }))
        .await;
    deleted.expect("the heal must read the stored peaks");

    assert!(
        matches!(
            &result,
            Err(ClientError::StoreError(StoreError::BlockHeaderNotFound(block)))
                if *block == BlockNumber::from(HEAL_HEIGHT)
        ),
        "the read must return the result of a new read, got {result:?}"
    );
    assert_eq!(rpc.proof_requests(), requests_before, "the heal must not request a proof");
    assert_eq!(stored_nodes(&store_path).await, nodes_before, "the heal must store no node");
}

/// A heal whose insert the store refuses returns the result of a new read, which still fails. The
/// heal does not insert again, and a later read repairs the store.
#[tokio::test]
async fn partial_mmr_heal_with_a_refused_insert_reads_again() {
    let HealSetup { client, node, store, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
    delete_upper_siblings(&store_path, NOTE_BLOCK);
    let nodes_before = stored_nodes(&store_path).await;

    store.inserts.refuse_next(1);
    let inserts_before = store.inserts.calls();
    let reads_before = store.partial_mmr_reads();
    let result = client.get_current_partial_mmr().await;
    assert!(is_inconsistent_mmr(&result), "the read must fail again, got {result:?}");
    assert_eq!(store.inserts.calls(), inserts_before + 1, "the heal must not insert again");
    assert_eq!(store.partial_mmr_reads(), reads_before + 2, "the heal must read again");
    assert_eq!(stored_nodes(&store_path).await, nodes_before);

    client
        .get_current_partial_mmr()
        .await
        .expect("a later read must repair the store");
    assert_full_path(&store_path, &node, NOTE_BLOCK, BlockNumber::from(HEAL_HEIGHT)).await;
}

/// A heal whose insert the store refuses after another client repaired the store returns the result
/// of a new read.
#[tokio::test]
async fn partial_mmr_heal_returns_the_new_read_after_a_refused_insert() {
    let HealSetup { client, node, store, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
    let other = client_on(&store_path, node.clone(), false).await;
    delete_upper_siblings(&store_path, NOTE_BLOCK);

    store.hold_insert_of(BlockNumber::from(NOTE_BLOCK));
    store.inserts.refuse_next(1);
    let inserts_before = store.inserts.calls();
    let (result, healed) = Box::pin(while_held(
        &store.insert_gate,
        client.get_current_partial_mmr(),
        other.get_current_partial_mmr(),
    ))
    .await;
    healed
        .expect("the heal must insert the missing nodes")
        .expect("the other client must repair the store");

    result.expect("the read must return the result of a new read");
    assert_eq!(store.inserts.calls(), inserts_before + 1);
    assert_full_path(&store_path, &node, NOTE_BLOCK, BlockNumber::from(HEAL_HEIGHT)).await;
}

/// A block that another writer untracks after the heal checked it, and before the heal stores its
/// missing nodes, stays untracked.
#[tokio::test]
async fn partial_mmr_heal_keeps_a_block_untracked_during_its_insert() {
    let HealSetup { client, store, store_path, .. } = heal_setup(&[NOTE_BLOCK]).await;
    delete_upper_siblings(&store_path, NOTE_BLOCK);

    store.hold_insert_of(BlockNumber::from(NOTE_BLOCK));
    let (result, untracked) =
        Box::pin(while_held(&store.insert_gate, client.get_current_partial_mmr(), async {
            untrack_with_sql(&store_path, NOTE_BLOCK);
        }))
        .await;
    untracked.expect("the heal must insert the missing nodes");

    let partial_mmr = result.expect("the read must succeed");
    assert!(!partial_mmr.is_tracked(NOTE_BLOCK as usize));
    let tracked = store.get_tracked_block_header_numbers().await.unwrap();
    assert!(!tracked.contains(&(NOTE_BLOCK as usize)), "the repair must not track the block");
}

/// A block whose header another writer deletes while the heal waits for its proof stops the heal.
/// The heal requests no proof for the other broken block, stores no node and returns the result of
/// a new read, which fails because the other block is still broken.
#[tokio::test]
async fn partial_mmr_heal_stops_for_a_block_header_deleted_during_its_proof_request() {
    let blocks = [NOTE_BLOCK, 6];
    let HealSetup { client, rpc, store, store_path, .. } = heal_setup(&blocks).await;
    for block in blocks {
        delete_nodes(&store_path, &sibling_indices(block, 2));
    }
    let nodes_before = stored_nodes(&store_path).await;

    rpc.proof_gate.arm();
    let requests_before = rpc.proof_requests();
    let inserts_before = store.inserts.calls();
    let reads_before = store.partial_mmr_reads();
    let (result, deleted) =
        Box::pin(while_held(&rpc.proof_gate, client.get_current_partial_mmr(), async {
            delete_header_with_sql(&store_path, NOTE_BLOCK);
        }))
        .await;
    deleted.expect("the heal must request the proof of block 1");

    assert!(
        is_missing_path_of(&result, 6),
        "the read must return the result of a new read, got {result:?}"
    );
    assert_eq!(rpc.proof_requests(), requests_before + 1, "the heal must not request block 6");
    assert_eq!(store.inserts.calls(), inserts_before, "the heal must not insert");
    assert_eq!(store.partial_mmr_reads(), reads_before + 2, "the heal must read again");
    assert_eq!(stored_nodes(&store_path).await, nodes_before, "the heal must store no node");
}

/// A block that another writer untracks while the heal waits for its proof stays untracked. The
/// heal requests no proof for the other broken block, stores no node and returns the result of a
/// new read, which fails because the other block is still broken.
#[tokio::test]
async fn partial_mmr_heal_writes_nothing_for_a_block_untracked_during_its_proof_request() {
    let blocks = [NOTE_BLOCK, 6];
    let HealSetup { client, rpc, store, store_path, .. } = heal_setup(&blocks).await;
    for block in blocks {
        delete_nodes(&store_path, &sibling_indices(block, 2));
    }
    let nodes_before = stored_nodes(&store_path).await;

    rpc.proof_gate.arm();
    let requests_before = rpc.proof_requests();
    let inserts_before = store.inserts.calls();
    let reads_before = store.partial_mmr_reads();
    let (result, untracked) =
        Box::pin(while_held(&rpc.proof_gate, client.get_current_partial_mmr(), async {
            untrack_with_sql(&store_path, NOTE_BLOCK);
        }))
        .await;
    untracked.expect("the heal must request the proof of block 1");

    assert!(
        is_missing_path_of(&result, 6),
        "the read must return the result of a new read, got {result:?}"
    );
    assert_eq!(rpc.proof_requests(), requests_before + 1, "the heal must not request block 6");
    assert_eq!(store.inserts.calls(), inserts_before, "the heal must not insert");
    assert_eq!(store.partial_mmr_reads(), reads_before + 2, "the heal must read again");
    assert_eq!(stored_nodes(&store_path).await, nodes_before, "the heal must store no node");
    let tracked = store.get_tracked_block_header_numbers().await.unwrap();
    assert!(!tracked.contains(&(NOTE_BLOCK as usize)), "the heal must not track block 1");
}
