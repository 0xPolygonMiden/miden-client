extern crate alloc;

mod client;
pub use client::{
    accounts::AccountTemplate, rpc, store_authenticator::StoreAuthenticator, sync::SyncSummary,
    Client, NoteConsumability, NoteRelevance,
};

pub mod config;
pub mod errors;
pub mod store;

// MIDEN BASE RE-EXPORTS
// ================================================================================================

pub mod accounts {
    pub use miden_objects::accounts::{
        Account, AccountCode, AccountData, AccountId, AccountStorage, AccountStorageType,
        AccountStub, AuthSecretKey,
    };

    #[cfg(feature = "testing")]
    pub mod testing {
        pub use miden_objects::accounts::account_id::testing::*;
    }
}

pub mod assembly {
    pub use miden_objects::assembly::{AstSerdeOptions, ModuleAst, ProgramAst};
}

pub mod assets {
    pub use miden_objects::assets::{Asset, AssetVault, FungibleAsset, TokenSymbol};
}

pub mod crypto {
    pub use miden_objects::crypto::{
        merkle::{
            InOrderIndex, LeafIndex, MerklePath, MmrDelta, MmrPeaks, MmrProof, SmtLeaf, SmtProof,
        },
        rand::{FeltRng, RpoRandomCoin},
    };
}

pub mod notes {
    pub use miden_objects::notes::{
        Note, NoteAssets, NoteExecutionHint, NoteId, NoteInclusionProof, NoteInputs, NoteMetadata,
        NoteRecipient, NoteScript, NoteTag, NoteType, Nullifier,
    };
}

pub use miden_objects::{BlockHeader, Digest, Felt, StarkField, Word};

pub mod transactions {
    pub use miden_objects::transaction::{
        ExecutedTransaction, InputNote, OutputNote, OutputNotes, ProvenTransaction, TransactionId,
        TransactionScript,
    };
    pub use miden_tx::{
        auth::TransactionAuthenticator,
        utils::{Deserializable, Serializable},
        DataStoreError, ScriptTarget, TransactionExecutorError,
    };

    pub use super::client::transactions::*;
}

#[cfg(all(test, feature = "executable"))]
pub mod mock;

#[cfg(all(test, feature = "executable"))]
pub mod tests;
