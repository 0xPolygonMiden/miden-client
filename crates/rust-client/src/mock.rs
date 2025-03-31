use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    vec::Vec,
};
use std::env::temp_dir;

use async_trait::async_trait;
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    Felt, Word,
    account::{AccountCode, AccountDelta, AccountId},
    asset::{FungibleAsset, NonFungibleAsset},
    block::{BlockHeader, BlockNumber, ProvenBlock},
    crypto::{
        merkle::{Mmr, MmrProof, SmtProof},
        rand::RpoRandomCoin,
    },
    note::{NoteId, NoteLocation, NoteTag, Nullifier},
    testing::{
        account_id::{ACCOUNT_ID_PRIVATE_FUNGIBLE_FAUCET, ACCOUNT_ID_PRIVATE_SENDER},
        note::NoteBuilder,
    },
    transaction::{InputNote, OutputNote, ProvenTransaction},
};
use miden_testing::MockChain;
use rand::{Rng, rngs::StdRng};
use tonic::Response;
use uuid::Uuid;

use crate::{
    Client,
    keystore::FilesystemKeyStore,
    rpc::{
        NodeRpcClient, RpcError,
        domain::{
            account::{AccountDetails, AccountProofs},
            note::{NetworkNote, NoteSyncInfo},
            nullifier::NullifierUpdate,
            sync::StateSyncInfo,
        },
        generated::{
            merkle::MerklePath,
            note::NoteSyncRecord,
            responses::{SyncNoteResponse, SyncStateResponse},
        },
    },
    store::sqlite_store::SqliteStore,
    transaction::ForeignAccount,
};

pub type MockClient = Client;

/// Mock RPC API
///
/// This struct implements the RPC API used by the client to communicate with the node. It is
/// intended to be used for testing purposes only.
#[derive(Clone)]
pub struct MockRpcApi {
    pub notes: BTreeMap<NoteId, InputNote>,
    pub blocks: Vec<ProvenBlock>,
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
        let mock_chain = MockChain::empty();
        let mut api = Self {
            notes: BTreeMap::new(),
            blocks: vec![],
            mock_chain,
        };

        let note_first = NoteBuilder::new(
            ACCOUNT_ID_PRIVATE_SENDER.try_into().unwrap(),
            RpoRandomCoin::new(Word::default()),
        )
        .add_assets([FungibleAsset::mock(20)])
        .build(&TransactionKernel::testing_assembler())
        .unwrap();

        let note_second = NoteBuilder::new(
            ACCOUNT_ID_PRIVATE_FUNGIBLE_FAUCET.try_into().unwrap(),
            RpoRandomCoin::new(Word::default()),
        )
        .add_assets([NonFungibleAsset::mock(&[1, 2, 3])])
        .build(&TransactionKernel::testing_assembler())
        .unwrap();

        api.seal_block(vec![], vec![]); // Block 0
        api.seal_block(vec![OutputNote::Full(note_first)], vec![]); // Block 1 - First note
        api.seal_block(vec![], vec![]); // Block 2
        api.seal_block(vec![], vec![]); // Block 3
        api.seal_block(vec![OutputNote::Full(note_second.clone())], vec![]); // Block 4 - Second note
        api.seal_block(vec![], vec![note_second.nullifier()]); // Block 5 - Second note nullifier

        // Collect the notes from the mock_chain
        api.notes = api
            .mock_chain
            .available_notes()
            .iter()
            .map(|n| (n.id(), n.clone().try_into().unwrap()))
            .collect();

        api
    }

    /// Seals a block with the given notes and nullifiers.
    fn seal_block(&mut self, notes: Vec<OutputNote>, nullifiers: Vec<Nullifier>) {
        for note in notes {
            self.mock_chain.add_pending_note(note);
        }

        for nullifier in nullifiers {
            self.mock_chain.add_nullifier(nullifier);
        }

        let block = self.mock_chain.seal_block(None, None);
        self.blocks.push(block);
    }

    /// Returns the current MMR of the blockchain.
    pub fn get_mmr(&self) -> Mmr {
        self.blocks.iter().map(ProvenBlock::commitment).into()
    }

    /// Retrieves the note at the specified position.
    pub fn get_note_at(&self, pos: usize) -> InputNote {
        self.notes.values().nth(pos).cloned().unwrap()
    }

    /// Returns the chain tip block number.
    fn get_chain_tip_block_num(&self) -> BlockNumber {
        self.blocks.last().map(|b| b.header().block_num()).unwrap()
    }

    /// Retrieves a block by its block number.
    fn get_block_by_num(&self, block_num: BlockNumber) -> Option<&ProvenBlock> {
        self.blocks.get(block_num.as_usize())
    }

    /// Generates a sync state response based on the request block number.
    pub fn get_sync_state_request(&self, request_block_num: BlockNumber) -> SyncStateResponse {
        // Determine the next block number to sync
        let next_block_num = self
            .notes
            .values()
            .filter_map(|n| n.location().map(NoteLocation::block_num))
            .filter(|&n| n > request_block_num)
            .min()
            .unwrap_or_else(|| self.get_chain_tip_block_num());

        // Retrieve the next block
        let Some(next_block) = self.get_block_by_num(next_block_num) else {
            return SyncStateResponse::default();
        };

        // Prepare the MMR delta
        let mmr_delta = self
            .get_mmr()
            .get_delta((request_block_num.as_u32() + 1) as usize, next_block_num.as_usize())
            .ok()
            .map(Into::into);

        // Collect notes that are in the next block
        let notes = self.get_notes_in_block(next_block_num).collect();

        SyncStateResponse {
            chain_tip: self.get_chain_tip_block_num().as_u32(),
            block_header: Some(next_block.header().into()),
            mmr_delta,
            accounts: vec![],
            transactions: vec![],
            notes,
        }
    }

    /// Retrieves notes that are included in the specified block number.
    fn get_notes_in_block(
        &self,
        block_num: BlockNumber,
    ) -> impl Iterator<Item = NoteSyncRecord> + '_ {
        self.notes.values().filter_map(move |note| {
            if note.location().is_some_and(|loc| loc.block_num() == block_num) {
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
        &self,
        _block_num: BlockNumber,
        _note_tags: &[NoteTag],
    ) -> Result<NoteSyncInfo, RpcError> {
        let response = SyncNoteResponse {
            chain_tip: u32::try_from(self.blocks.len()).expect("block number overflow"),
            notes: vec![],
            block_header: Some(self.blocks.last().unwrap().header().into()),
            mmr_path: Some(MerklePath::default()),
        };
        let response = Response::new(response.clone());
        response.into_inner().try_into()
    }

    /// Executes the specified sync state request and returns the response.
    async fn sync_state(
        &self,
        block_num: BlockNumber,
        _account_ids: &[AccountId],
        _note_tags: &[NoteTag],
    ) -> Result<StateSyncInfo, RpcError> {
        // Match request -> response through block_num
        let response = self.get_sync_state_request(block_num);

        Ok(response.try_into().unwrap())
    }

    /// Creates and executes a [GetBlockHeaderByNumberRequest]. Will retrieve the block header
    /// for the specified block number. If the block number is not provided, the chain tip block
    /// header will be returned.
    async fn get_block_header_by_number(
        &self,
        mut block_num: Option<BlockNumber>,
        include_mmr_proof: bool,
    ) -> Result<(BlockHeader, Option<MmrProof>), RpcError> {
        if block_num.is_none() {
            block_num = Some(self.get_chain_tip_block_num());
        }

        if block_num == Some(0.into()) {
            return Ok((self.blocks.first().unwrap().header().clone(), None));
        }
        let block = self
            .blocks
            .iter()
            .find(|b| b.header().block_num() == block_num.unwrap())
            .unwrap();

        let mmr_proof = if include_mmr_proof {
            Some(self.get_mmr().open(block_num.unwrap().as_usize()).unwrap())
        } else {
            None
        };

        Ok((block.header().clone(), mmr_proof))
    }

    async fn get_notes_by_id(&self, note_ids: &[NoteId]) -> Result<Vec<NetworkNote>, RpcError> {
        // assume all private notes for now
        let hit_notes = note_ids.iter().filter_map(|id| self.notes.get(id));
        let mut return_notes = vec![];
        for note in hit_notes {
            return_notes.push(NetworkNote::Private(
                note.id(),
                *note.note().metadata(),
                note.proof().expect("Note should have an inclusion proof").clone(),
            ));
        }
        Ok(return_notes)
    }

    async fn submit_proven_transaction(
        &self,
        _proven_transaction: ProvenTransaction,
    ) -> std::result::Result<(), RpcError> {
        // TODO: add some basic validations to test error cases
        Ok(())
    }

    async fn get_account_details(
        &self,
        _account_id: AccountId,
    ) -> Result<AccountDetails, RpcError> {
        panic!("shouldn't be used for now")
    }

    async fn get_account_proofs(
        &self,
        _: &BTreeSet<ForeignAccount>,
        _code_commitments: Vec<AccountCode>,
    ) -> Result<AccountProofs, RpcError> {
        // TODO: Implement fully
        Ok((self.blocks.last().unwrap().header().block_num(), vec![]))
    }

    async fn check_nullifiers_by_prefix(
        &self,
        _prefix: &[u16],
        _block_num: BlockNumber,
    ) -> Result<Vec<NullifierUpdate>, RpcError> {
        // Always return an empty list for now since it's only used when importing
        Ok(vec![])
    }

    async fn check_nullifiers(&self, _nullifiers: &[Nullifier]) -> Result<Vec<SmtProof>, RpcError> {
        unimplemented!("shouldn't be used for now")
    }

    async fn get_account_state_delta(
        &self,
        _account_id: AccountId,
        _from_block: BlockNumber,
        _to_block: BlockNumber,
    ) -> Result<AccountDelta, RpcError> {
        unimplemented!("shouldn't be used for now")
    }

    async fn get_block_by_number(&self, block_num: BlockNumber) -> Result<ProvenBlock, RpcError> {
        let block = self
            .blocks
            .iter()
            .find(|b| b.header().block_num() == block_num)
            .unwrap()
            .clone();

        Ok(block)
    }
}

// HELPERS
// ================================================================================================

pub async fn create_test_client() -> (MockClient, MockRpcApi, FilesystemKeyStore<StdRng>) {
    let store = SqliteStore::new(create_test_store_path()).await.unwrap();
    let store = Arc::new(store);

    let mut rng = rand::rng();
    let coin_seed: [u64; 4] = rng.random();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));

    let keystore = FilesystemKeyStore::new(temp_dir()).unwrap();

    let rpc_api = MockRpcApi::new();
    let arc_rpc_api = Arc::new(rpc_api.clone());

    let client =
        MockClient::new(arc_rpc_api, Box::new(rng), store, Arc::new(keystore.clone()), true, None);
    (client, rpc_api, keystore)
}

pub fn create_test_store_path() -> std::path::PathBuf {
    let mut temp_file = temp_dir();
    temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
    temp_file
}
