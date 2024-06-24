extern crate alloc;

mod client;
pub use client::{
    accounts::AccountTemplate, rpc, store_authenticator::StoreAuthenticator, sync::SyncSummary,
    transactions, Client, NoteConsumability, NoteRelevance,
};

pub mod config;
pub mod errors;
pub mod store;

/// Miden Base re-exports
pub mod objects {
    pub use miden_objects::{
        accounts::{Account, AccountData, AccountId, AccountStorageType, AuthSecretKey},
        assembly::ProgramAst,
        assets::{Asset, FungibleAsset, TokenSymbol},
        crypto::{
            merkle::{InOrderIndex, MerklePath, MmrDelta, MmrPeaks, MmrProof},
            rand::{FeltRng, RpoRandomCoin},
        },
        notes::{
            Note, NoteAssets, NoteExecutionHint, NoteId, NoteInputs, NoteMetadata, NoteRecipient,
            NoteScript, NoteTag, NoteType,
        },
        transaction::{InputNote, ProvenTransaction, TransactionId},
        BlockHeader, Digest, Felt, Word,
    };

    #[cfg(feature = "testing")]
    pub mod testing {
        pub use miden_objects::accounts::account_id::testing::*;
    }
}

pub mod tx {
    pub use miden_tx::{
        auth::TransactionAuthenticator,
        utils::{Deserializable, Serializable},
        DataStoreError, ScriptTarget, TransactionExecutorError,
    };
}

#[cfg(all(test, feature = "executable"))]
pub mod mock;

#[cfg(all(test, feature = "executable"))]
pub mod tests;
