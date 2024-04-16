// use miden_objects::{
//     accounts::Account,
//     crypto::dsa::rpo_falcon512::KeyPair,
//     Word,
// };
// use miden_tx::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};

use async_trait::async_trait;

// Hoping that eventually, we can use the generic store type defined in client/store/mod.rs.
// Might need to add conditional rendering to add async_trait to the trait definitions? 
// Or maybe we can just add it to the trait definitions and it will work for both the CLI and the browser?
// Basically, can we keep the generic Client definition and the Store trait definitions the same
// and add what we need for the browser-specific implementation
#[async_trait(?Send)]
pub trait Store {
    // TEST
    // --------------------------------------------------------------------------------------------

    async fn insert_string(
        &mut self,
        data: String
    ) -> Result<(), ()>;

    // SYNC
    // --------------------------------------------------------------------------------------------

    // async fn get_note_tags(
    //     &self
    // ) -> Result<Vec<u64>, ()>;

    // async fn add_note_tag(
    //     &mut self,
    //     tag: u64,
    // ) -> Result<bool, ()>;

    // async fn get_sync_height(
    //     &self
    // ) -> Result<u32, ()>;

    // async fn apply_state_sync(
    //     &mut self,
    //     block_header: BlockHeader,
    //     nullifiers: Vec<Digest>,
    //     committed_notes: Vec<(NoteId, NoteInclusionProof)>,
    //     committed_transactions: &[TransactionId],
    //     new_mmr_peaks: MmrPeaks,
    //     new_authentication_nodes: &[(InOrderIndex, Digest)],
    // ) -> Result<(), ()>;

    // TRANSACTIONS
    // --------------------------------------------------------------------------------------------

    // async fn get_transactions(
    //     &mut self,
    //     filter: NativeTransactionFilter,
    // ) -> Result<Vec<TransactionRecord>, ()>;

    // async fn apply_transaction(
    //     &mut self,
    //     tx_result: TransactionResult,
    // ) -> Result<(), ()>;

    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

    // async fn get_account_ids(
    //     &self
    // ) -> Result<Vec<AccountId>, ()>;

    // async fn get_account_stubs(
    //     &self
    // ) -> Result<Vec<(AccountStub, Option<Word>)>, ()>;

    // async fn get_account_stub(
    //     &self,
    //     account_id: AccountId,
    // ) -> Result<(AccountStub, Option<Word>), ()>;

    // async fn get_account(
    //     &self,
    //     account_id: AccountId,
    // ) -> Result<(Account, Option<Word>), ()>;

    // async fn get_account_auth(
    //     &self,
    //     account_id: AccountId,
    // ) -> Result<AuthInfo, ()>;

    // async fn insert_account(
    //     &mut self,
    //     account: &Account,
    //     account_seed: Option<Word>,
    //     auth_info: &AuthInfo,
    // ) -> Result<(), ()>;

    // NOTES
    // --------------------------------------------------------------------------------------------

    // async fn get_input_notes(
    //     &mut self,
    //     filter: NativeNoteFilter,
    // ) -> Result<Vec<InputNoteRecord>, ()>;

    // async fn get_input_note(
    //     &self,
    //     note_id: NoteId,
    // ) -> Result<InputNoteRecord, ()>;

    // async fn insert_input_note(
    //     &mut self,
    //     note: &InputNoteRecord,
    // ) -> Result<(), ()>;

    // async fn get_output_notes(
    //     &self,
    //     filter: NativeNoteFilter,
    // ) -> Result<Vec<InputNoteRecord>, ()>;

    // async fn get_unspent_input_note_nullifiers(
    //     &self
    // ) -> Result<Vec<Nullifier>, ()>;
}

// DATABASE AUTH INFO
// ================================================================================================

// #[derive(Debug)]
// pub enum AuthInfo {
//     RpoFalcon512(KeyPair),
// }

// const RPO_FALCON512_AUTH: u8 = 0;

// impl AuthInfo {
//     /// Returns byte identifier of specific AuthInfo
//     const fn type_byte(&self) -> u8 {
//         match self {
//             AuthInfo::RpoFalcon512(_) => RPO_FALCON512_AUTH,
//         }
//     }
// }

// impl Serializable for AuthInfo {
//     fn write_into<W: ByteWriter>(
//         &self,
//         target: &mut W,
//     ) {
//         let mut bytes = vec![self.type_byte()];
//         match self {
//             AuthInfo::RpoFalcon512(key_pair) => {
//                 bytes.append(&mut key_pair.to_bytes());
//                 target.write_bytes(&bytes);
//             },
//         }
//     }
// }

// impl Deserializable for AuthInfo {
//     fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
//         let auth_type: u8 = source.read_u8()?;
//         match auth_type {
//             RPO_FALCON512_AUTH => {
//                 let key_pair = KeyPair::read_from(source)?;
//                 Ok(AuthInfo::RpoFalcon512(key_pair))
//             },
//             val => Err(DeserializationError::InvalidValue(val.to_string())),
//         }
//     }
// }

pub enum NativeTransactionFilter {
    /// Return all transactions.
    All,
    /// Filter by transactions that have not yet been committed to the blockchain as per the last
    /// sync.
    Uncomitted,
}

pub enum NativeNoteFilter {
    /// Return a list of all [InputNoteRecord].
    All,
    /// Filter by consumed [InputNoteRecord]. notes that have been used as inputs in transactions.
    Consumed,
    /// Return a list of committed [InputNoteRecord]. These represent notes that the blockchain
    /// has included in a block, and for which we are storing anchor data.
    Committed,
    /// Return a list of pending [InputNoteRecord]. These represent notes for which the store
    /// does not have anchor data.
    Pending,
}