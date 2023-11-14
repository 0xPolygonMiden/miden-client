use std::collections::BTreeMap;

use super::{errors::StoreError, AccountStub, ClientConfig};
use crypto::{hash::rpo::Rpo256, Felt, Word};
use objects::{
    accounts::{Account, AccountCode, AccountStorage, AccountVault},
    assets::Asset,
};
use rusqlite::{params, Connection};

mod migrations;

// CLIENT STORE
// ================================================================================================

pub struct Store {
    db: Connection,
}

impl Store {
    pub fn new(config: StoreConfig) -> Result<Self, StoreError> {
        let mut db = Connection::open(config.path).map_err(StoreError::ConnectionError)?;
        migrations::update_to_latest(&mut db)?;

        Ok(Self { db })
    }

    pub fn get_accounts(&self) -> Result<Vec<AccountStub>, StoreError> {
        let mut stmt = self
            .db
            .prepare("SELECT id, nonce FROM accounts")
            .map_err(StoreError::QueryError)?;

        let mut rows = stmt.query([]).map_err(StoreError::QueryError)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(StoreError::QueryError)? {
            // TODO: implement proper error handling and conversions

            // NOTE: the i64->u64 conversion is necessary when going in an out from sqlite,
            // as it has no native u64 type (only i64), so it can go out of range
            let id: i64 = row.get(0).unwrap();
            let id = u64::from_be_bytes(id.to_be_bytes());

            let nonce: u64 = row.get(1).unwrap();

            result.push(AccountStub::new(
                id.try_into().unwrap(),
                nonce.into(),
                Rpo256::hash_elements(&[Felt::new(2)]),
                Rpo256::hash_elements(&[Felt::new(3)]),
                Rpo256::hash_elements(&[Felt::new(4)]),
            ));
        }

        Ok(result)
    }

    pub fn insert_account(&self, account: &Account) -> Result<(), StoreError> {
        let id: u64 = account.id().into();
        let code_root = serde_json::to_string(&account.code().root())
            .map_err(StoreError::InputSerializationError)?;
        let storage_root = serde_json::to_string(&account.storage().root())
            .map_err(StoreError::InputSerializationError)?;
        let vault_root = serde_json::to_string(&account.vault().commitment())
            .map_err(StoreError::InputSerializationError)?;

        self.db.execute(
            "INSERT INTO accounts (id, code_root, storage_root, vault_root, nonce, committed) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                i64::from_be_bytes(id.to_be_bytes()),
                code_root,
                storage_root,
                vault_root,
                account.nonce().inner(),
                account.is_on_chain(),
            ],
        )
        .map(|_| ())
        .map_err(StoreError::QueryError)
    }

    pub fn insert_account_code(&self, account_code: &AccountCode) -> Result<(), StoreError> {
        let code_root = serde_json::to_string(&account_code.root())
            .map_err(StoreError::InputSerializationError)?;
        let code = serde_json::to_string(account_code.procedures())
            .map_err(StoreError::InputSerializationError)?;
        // ModuleAst does not derive Serialize
        let module = ""; // serde_json::to_string(account_code.module()).unwrap();

        self.db
            .execute(
                "INSERT INTO account_code (root, procedures, module) VALUES (?, ?, ?)",
                params![code_root, code, module,],
            )
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    pub fn insert_account_storage(
        &self,
        account_storage: &AccountStorage,
    ) -> Result<(), StoreError> {
        let storage_root = serde_json::to_string(&account_storage.root())
            .map_err(StoreError::InputSerializationError)?;

        let storage_slots: BTreeMap<u64, &Word> = account_storage.slots().leaves().collect();
        let storage_slots =
            serde_json::to_string(&storage_slots).map_err(StoreError::InputSerializationError)?;

        self.db
            .execute(
                "INSERT INTO account_storage (root, slots) VALUES (?, ?)",
                params![storage_root, storage_slots],
            )
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    pub fn insert_account_vault(&self, account_vault: &AccountVault) -> Result<(), StoreError> {
        let vault_root = serde_json::to_string(&account_vault.commitment())
            .map_err(StoreError::InputSerializationError)?;

        let assets: Vec<Asset> = account_vault.assets().collect();
        let assets = serde_json::to_string(&assets).map_err(StoreError::InputSerializationError)?;

        self.db
            .execute(
                "INSERT INTO account_vault (root, assets) VALUES (?, ?)",
                params![vault_root, assets],
            )
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }
}

// STORE CONFIG
// ================================================================================================

pub struct StoreConfig {
    path: String,
}

impl From<&ClientConfig> for StoreConfig {
    fn from(config: &ClientConfig) -> Self {
        Self {
            path: config.store_path.clone(),
        }
    }
}

mod tests {
    // TODO: Add tests
}
