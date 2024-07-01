extern crate alloc;

mod client;
pub use client::{rpc, sync::SyncSummary, Client};

pub mod config;
pub mod errors;
pub mod store;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

// RE-EXPORTS
// ================================================================================================

pub mod accounts {
    pub use miden_objects::accounts::{
        Account, AccountCode, AccountData, AccountId, AccountStorage, AccountStorageType,
        AccountStub, AccountType, StorageSlotType,
    };

    pub use super::client::accounts::AccountTemplate;
}

pub mod assembly {
    pub use miden_objects::assembly::{AstSerdeOptions, ModuleAst, ProgramAst};
}

pub mod assets {
    pub use miden_objects::assets::{Asset, AssetVault, FungibleAsset, TokenSymbol};
}

pub mod auth {
    pub use miden_objects::accounts::AuthSecretKey;
    pub use miden_tx::auth::TransactionAuthenticator;

    pub use super::client::store_authenticator::StoreAuthenticator;
}

pub mod blocks {
    pub use miden_objects::BlockHeader;
}

pub mod crypto {
    pub use miden_objects::{
        crypto::{
            merkle::{
                InOrderIndex, LeafIndex, MerklePath, MmrDelta, MmrPeaks, MmrProof, SmtLeaf,
                SmtProof,
            },
            rand::{FeltRng, RpoRandomCoin},
        },
        Digest, ONE, ZERO,
    };
}

pub use miden_objects::{Felt, StarkField, Word};

pub mod notes {
    pub use miden_objects::notes::{
        Note, NoteAssets, NoteExecutionHint, NoteFile, NoteId, NoteInclusionProof, NoteInputs,
        NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType, Nullifier,
    };

    pub use super::client::{NoteConsumability, NoteRelevance};
}

pub mod transactions {
    pub use miden_objects::transaction::{
        ExecutedTransaction, InputNote, OutputNote, OutputNotes, ProvenTransaction, TransactionId,
        TransactionScript,
    };
    pub use miden_tx::{DataStoreError, ScriptTarget, TransactionExecutorError};

    pub use super::client::transactions::*;
}

pub mod utils {
    pub use miden_tx::utils::{
        bytes_to_hex_string, ByteReader, ByteWriter, Deserializable, DeserializationError,
        Serializable,
    };
}

#[cfg(feature = "testing")]
pub mod testing {
    pub use miden_objects::accounts::account_id::testing::*;
}
