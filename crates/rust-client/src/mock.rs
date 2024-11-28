use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    vec::Vec,
};
use std::env::temp_dir;

use async_trait::async_trait;
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_OFF_CHAIN_SENDER,
        },
        AccountId,
    },
    assets::{FungibleAsset, NonFungibleAsset},
    block::Block,
    crypto::{
        merkle::{Mmr, MmrProof},
        rand::RpoRandomCoin,
    },
    notes::{Note, NoteId, NoteTag},
    testing::notes::NoteBuilder,
    transaction::{InputNote, ProvenTransaction},
    BlockHeader, Digest, Felt, Word,
};
use miden_tx::testing::mock_chain::MockChain;
use rand::Rng;
use tonic::Response;
use uuid::Uuid;

use crate::{
    rpc::{
        domain::{
            accounts::{AccountDetails, AccountProofs},
            notes::{NoteDetails, NoteInclusionDetails, NoteSyncInfo},
            sync::StateSyncInfo,
        },
        generated::{
            note::NoteSyncRecord,
            responses::{NullifierUpdate, SyncNoteResponse, SyncStateResponse},
        },
        NodeRpcClient, RpcError,
    },
    store::{sqlite_store::SqliteStore, StoreAuthenticator},
    Client,
};

pub type MockClient = Client<RpoRandomCoin>;

/// Mock RPC API
///
/// This struct implements the RPC API used by the client to communicate with the node. It is
/// intended to be used for testing purposes only.
#[derive(Clone)]
pub struct MockRpcApi {
    pub notes: BTreeMap<NoteId, InputNote>,
    pub blocks: Vec<Block>,
    pub mock_chain: MockChain,
}
impl Default for MockRpcApi {
    fn default() -> Self {
        Self::new()
    }
}
impl MockRpcApi {
    /// Creates a new `MockRpcApi` instance with pre-populated blocks and notes.
    pub fn new() -> Self {
        let mock_chain = MockChain::new();
        let mut api = Self {
            notes: BTreeMap::new(),
            blocks: vec![],
            mock_chain,
        };

        let note_first = NoteBuilder::new(
            ACCOUNT_ID_OFF_CHAIN_SENDER.try_into().unwrap(),
            RpoRandomCoin::new(Word::default()),
        )
        .add_assets([FungibleAsset::mock(20)])
        .build(&TransactionKernel::testing_assembler())
        .unwrap();

        let note_second = NoteBuilder::new(
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN.try_into().unwrap(),
            RpoRandomCoin::new(Word::default()),
        )
        .add_assets([NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN, &[1, 2, 3])])
        .build(&TransactionKernel::testing_assembler())
        .unwrap();

        api.seal_block(vec![], vec![]); // Block 0
        api.seal_block(vec![note_first], vec![]); // Block 1 - First note
        api.seal_block(vec![], vec![]); // Block 2
        api.seal_block(vec![], vec![]); // Block 3
        api.seal_block(vec![note_second.clone()], vec![]); // Block 4 - Second note
        api.seal_block(vec![], vec![note_second.nullifier()]); // Block 5 - Second note nullifier

        // Collect the notes from the mock_chain
        api.notes = api.mock_chain.available_notes().iter().map(|n| (n.id(), n.clone())).collect();

        api
    }

    /// Seals a block with the given notes and nullifiers.
    fn seal_block(&mut self, notes: Vec<Note>, nullifiers: Vec<miden_objects::notes::Nullifier>) {
        for note in notes {
            self.mock_chain.add_note(note);
        }

        for nullifier in nullifiers {
            self.mock_chain.add_nullifier(nullifier);
        }

        let block = self.mock_chain.seal_block(None);
        self.blocks.push(block);
    }

    /// Returns the current MMR of the blockchain.
    pub fn get_mmr(&self) -> Mmr {
        self.blocks.iter().map(Block::hash).into()
    }

    /// Retrieves the note at the specified position.
    pub fn get_note_at(&self, pos: usize) -> InputNote {
        self.notes.values().nth(pos).cloned().unwrap()
    }

    /// Returns the chain tip block number.
    fn get_chain_tip_block_num(&self) -> u32 {
        self.blocks.last().map(|b| b.header().block_num()).unwrap()
    }

    /// Retrieves a block by its block number.
    fn get_block_by_num(&self, block_num: u32) -> Option<&Block> {
        self.blocks.get(block_num as usize)
    }

    /// Generates a sync state response based on the request block number.
    pub fn get_sync_state_request(&self, request_block_num: u32) -> SyncStateResponse {
        // Determine the next block number to sync
        let next_block_num = self
            .notes
            .values()
            .filter_map(|n| n.location().map(|loc| loc.block_num()))
            .filter(|&n| n > request_block_num)
            .min()
            .unwrap_or_else(|| self.get_chain_tip_block_num());

        // Retrieve the next block
        let next_block = match self.get_block_by_num(next_block_num) {
            Some(block) => block,
            None => return SyncStateResponse::default(), // Return default if block not found
        };

        // Prepare the MMR delta
        let mmr_delta = self
            .get_mmr()
            .get_delta((request_block_num + 1) as usize, next_block_num as usize)
            .ok()
            .map(Into::into);

        // Collect notes that are in the next block
        let notes = self.get_notes_in_block(next_block_num).collect();

        // Collect nullifiers from the next block
        let nullifiers = next_block
            .nullifiers()
            .iter()
            .map(|n| NullifierUpdate {
                nullifier: Some(n.inner().into()),
                block_num: next_block_num,
            })
            .collect();

        SyncStateResponse {
            chain_tip: self.get_chain_tip_block_num(),
            block_header: Some(next_block.header().into()),
            mmr_delta,
            accounts: vec![],
            transactions: vec![],
            notes,
            nullifiers,
        }
    }

    /// Retrieves notes that are included in the specified block number.
    fn get_notes_in_block(&self, block_num: u32) -> impl Iterator<Item = NoteSyncRecord> + '_ {
        self.notes.values().filter_map(move |note| {
            if note.location().map_or(false, |loc| loc.block_num() == block_num) {
                let proof = note.proof()?;
                Some(NoteSyncRecord {
                    note_index: 0,
                    note_id: Some(note.id().into()),
                    metadata: Some((*note.note().metadata()).into()),
                    merkle_path: Some(proof.note_path().clone().into()),
                })
            } else {
                None
            }
        })
    }
}
use alloc::boxed::Box;
#[async_trait(?Send)]
impl NodeRpcClient for MockRpcApi {
    async fn sync_notes(
        &mut self,
        _block_num: u32,
        _note_tags: &[NoteTag],
    ) -> Result<NoteSyncInfo, RpcError> {
        let response = SyncNoteResponse {
            chain_tip: self.blocks.len() as u32,
            notes: vec![],
            block_header: Some(self.blocks.last().unwrap().header().into()),
            mmr_path: Some(Default::default()),
        };
        let response = Response::new(response.clone());
        response.into_inner().try_into()
    }

    /// Executes the specified sync state request and returns the response.
    async fn sync_state(
        &mut self,
        block_num: u32,
        _account_ids: &[AccountId],
        _note_tags: &[NoteTag],
        _nullifiers_tags: &[u16],
    ) -> Result<StateSyncInfo, RpcError> {
        // Match request -> response through block_num
        let response = self.get_sync_state_request(block_num);

        Ok(response.try_into().unwrap())
    }

    /// Creates and executes a [GetBlockHeaderByNumberRequest].
    /// Only used for retrieving genesis block right now so that's the only case we need to cover.
    async fn get_block_header_by_number(
        &mut self,
        block_num: Option<u32>,
        include_mmr_proof: bool,
    ) -> Result<(BlockHeader, Option<MmrProof>), RpcError> {
        if block_num == Some(0) {
            return Ok((self.blocks.first().unwrap().header(), None));
        }
        let block = self
            .blocks
            .iter()
            .find(|b| b.header().block_num() == block_num.unwrap())
            .unwrap();

        let mmr_proof = if include_mmr_proof {
            Some(self.get_mmr().open(block_num.unwrap() as usize).unwrap())
        } else {
            None
        };

        Ok((block.header(), mmr_proof))
    }

    async fn get_notes_by_id(&mut self, note_ids: &[NoteId]) -> Result<Vec<NoteDetails>, RpcError> {
        // assume all off-chain notes for now
        let hit_notes = note_ids.iter().filter_map(|id| self.notes.get(id));
        let mut return_notes = vec![];
        for note in hit_notes {
            let inclusion_details = NoteInclusionDetails::new(
                note.proof()
                    .expect("Note should have an inclusion proof")
                    .location()
                    .block_num(),
                note.proof()
                    .expect("Note should have an inclusion proof")
                    .location()
                    .node_index_in_block(),
                note.proof().expect("Note should have an inclusion proof").note_path().clone(),
            );
            return_notes.push(NoteDetails::Private(
                note.id(),
                *note.note().metadata(),
                inclusion_details,
            ));
        }
        Ok(return_notes)
    }

    async fn submit_proven_transaction(
        &mut self,
        _proven_transaction: ProvenTransaction,
    ) -> std::result::Result<(), RpcError> {
        // TODO: add some basic validations to test error cases
        Ok(())
    }

    async fn get_account_update(
        &mut self,
        _account_id: AccountId,
    ) -> Result<AccountDetails, RpcError> {
        panic!("shouldn't be used for now")
    }

    async fn get_account_proofs(
        &mut self,
        _account_ids: &BTreeSet<AccountId>,
        _code_commitments: &[Digest],
        _include_headers: bool,
    ) -> Result<AccountProofs, RpcError> {
        // TODO: Implement fully
        Ok((self.blocks.last().unwrap().header().block_num(), vec![]))
    }

    async fn check_nullifiers_by_prefix(
        &mut self,
        _prefix: &[u16],
    ) -> Result<Vec<(miden_objects::notes::Nullifier, u32)>, RpcError> {
        // Always return an empty list for now since it's only used when importing
        Ok(vec![])
    }
}

// HELPERS
// ================================================================================================

pub async fn create_test_client() -> (MockClient, MockRpcApi) {
    let store = SqliteStore::new(create_test_store_path()).await.unwrap();
    let store = Arc::new(store);

    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));

    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
    let rpc_api = MockRpcApi::new();
    let boxed_rpc_api = Box::new(rpc_api.clone());

    let client = MockClient::new(boxed_rpc_api, rng, store, Arc::new(authenticator), true);
    (client, rpc_api)
}

pub fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}
