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
        accounts::{
            Account, AccountCode, AccountData, AccountId, AccountStorage, AccountStorageType,
            AccountStub, AuthSecretKey,
        },
        assembly::{AstSerdeOptions, ModuleAst, ProgramAst},
        assets::{Asset, AssetVault, FungibleAsset, TokenSymbol},
        crypto::{
            merkle::{
                InOrderIndex, LeafIndex, MerklePath, MmrDelta, MmrPeaks, MmrProof, SmtLeaf,
                SmtProof,
            },
            rand::{FeltRng, RpoRandomCoin},
        },
        notes::{
            Note, NoteAssets, NoteExecutionHint, NoteId, NoteInclusionProof, NoteInputs,
            NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType, Nullifier,
        },
        transaction::{
            InputNote, OutputNotes, ProvenTransaction, TransactionId, TransactionScript,
        },
        BlockHeader, Digest, Felt, StarkField, Word,
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
