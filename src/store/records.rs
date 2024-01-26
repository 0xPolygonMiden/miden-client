use super::accounts::SerializedAccountsParts;
use crate::errors::StoreError;
use crypto::{utils::Deserializable, Word};
use objects::{accounts::AccountId, Digest, Felt};

/// This module is meant to be store YYYRecord structs of the data that we fetch via queries

/// ACCOUNT RECORD
pub struct AccountRecord {
    id: AccountId,
    nonce: Felt,
    vault_root: Digest,
    storage_root: Digest,
    code_root: Digest,
    account_seed: Word,
    account_hash: Digest,
}

impl AccountRecord {
    /// Parse an account record from the provided parts.
    pub fn new(
        serialized_account_parts: SerializedAccountsParts,
    ) -> Result<AccountRecord, StoreError> {
        let (id, nonce, vault_root, storage_root, code_root, account_hash, account_seed) =
            serialized_account_parts;
        let account_seed_word: Word =
            Word::read_from_bytes(&account_seed).map_err(StoreError::DataDeserializationError)?;

        Ok(Self {
            id: (id as u64)
                .try_into()
                .expect("Conversion from stored AccountID should not panic"),
            nonce: (nonce as u64).into(),
            vault_root: serde_json::from_str(&vault_root)
                .map_err(StoreError::JsonDataDeserializationError)?,
            storage_root: Digest::try_from(&storage_root).map_err(StoreError::HexParseError)?,
            code_root: Digest::try_from(&code_root).map_err(StoreError::HexParseError)?,
            account_hash: Digest::try_from(&account_hash).map_err(StoreError::HexParseError)?,
            account_seed: account_seed_word,
        })
    }

    pub fn id(&self) -> AccountId {
        self.id
    }

    pub fn nonce(&self) -> Felt {
        self.nonce
    }

    pub fn vault_root(&self) -> Digest {
        self.vault_root
    }

    pub fn storage_root(&self) -> Digest {
        self.storage_root
    }

    pub fn code_root(&self) -> Digest {
        self.code_root
    }

    pub fn account_seed(&self) -> Word {
        self.account_seed
    }

    pub fn account_hash(&self) -> Digest {
        self.account_hash
    }
}
