use super::{errors::StoreError, AccountStub, ClientConfig};
use crypto::{
    dsa::rpo_falcon512::KeyPair,
    utils::{collections::BTreeMap, Deserializable, Serializable},
    Word,
};
use objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault},
    assembly::AstSerdeOptions,
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
            .prepare("SELECT id, nonce, vault_root, storage_root, code_root FROM accounts")
            .map_err(StoreError::QueryError)?;

        let mut rows = stmt.query([]).map_err(StoreError::QueryError)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(StoreError::QueryError)? {
            // TODO: implement proper error handling and conversions

            let id: i64 = row.get(0).map_err(StoreError::QueryError)?;
            let nonce: i64 = row.get(1).map_err(StoreError::QueryError)?;

            let vault_root: String = row.get(2).map_err(StoreError::QueryError)?;
            let storage_root: String = row.get(3).map_err(StoreError::QueryError)?;
            let code_root: String = row.get(4).map_err(StoreError::QueryError)?;

            result.push(AccountStub::new(
                (id as u64)
                    .try_into()
                    .expect("Conversion from stored AccountID should not panic"),
                (nonce as u64).into(),
                serde_json::from_str(&vault_root).map_err(StoreError::DataDeserializationError)?,
                serde_json::from_str(&storage_root)
                    .map_err(StoreError::DataDeserializationError)?,
                serde_json::from_str(&code_root).map_err(StoreError::DataDeserializationError)?,
            ));
        }

        Ok(result)
    }

    pub fn get_account_by_id(&self, account_id: AccountId) -> Result<AccountStub, StoreError> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT id, nonce, vault_root, storage_root, code_root FROM accounts WHERE id = ?",
            )
            .map_err(StoreError::QueryError)?;
        let account_id: u64 = account_id.into();

        let mut rows = stmt
            .query(params![account_id as i64])
            .map_err(StoreError::QueryError)?;

        if let Some(row) = rows.next().map_err(StoreError::QueryError)? {
            let id: i64 = row.get(0).map_err(StoreError::QueryError)?;
            let nonce: u64 = row.get(1).map_err(StoreError::QueryError)?;
            let vault_root: String = row.get(2).map_err(StoreError::QueryError)?;
            let storage_root: String = row.get(3).map_err(StoreError::QueryError)?;
            let code_root: String = row.get(4).map_err(StoreError::QueryError)?;

            let account = AccountStub::new(
                (id as u64)
                    .try_into()
                    .expect("Conversion from stored AccountID should not panic"),
                nonce.into(),
                serde_json::from_str(&vault_root).map_err(StoreError::DataDeserializationError)?,
                serde_json::from_str(&storage_root)
                    .map_err(StoreError::DataDeserializationError)?,
                serde_json::from_str(&code_root).map_err(StoreError::DataDeserializationError)?,
            );

            Ok(account)
        } else {
            Err(StoreError::AccountDataNotFound)
        }
    }

    pub fn get_account_keys(&self, account_id: AccountId) -> Result<KeyPair, StoreError> {
        let mut stmt = self
            .db
            .prepare("SELECT key_pair FROM account_keys WHERE account_id = ?")
            .map_err(StoreError::QueryError)?;
        let account_id: u64 = account_id.into();

        let mut rows = stmt
            .query(params![account_id as i64])
            .map_err(StoreError::QueryError)?;

        if let Some(row) = rows.next().map_err(StoreError::QueryError)? {
            let key_pair_bytes: Vec<u8> = row.get(0).map_err(StoreError::QueryError)?;
            let key_pair: KeyPair = KeyPair::read_from_bytes(&key_pair_bytes).unwrap();

            Ok(key_pair)
        } else {
            Err(StoreError::AccountDataNotFound)
        }
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
                id as i64,
                code_root,
                storage_root,
                vault_root,
                account.nonce().inner() as i64,
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
        let module = account_code.module().to_bytes(AstSerdeOptions {
            serialize_imports: true,
        });

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
                "INSERT INTO account_vaults (root, assets) VALUES (?, ?)",
                params![vault_root, assets],
            )
            .map(|_| ())
            .map_err(StoreError::QueryError)
    }

    pub fn insert_account_keys(
        &self,
        account_id: AccountId,
        key_pair: &KeyPair,
    ) -> Result<(), StoreError> {
        let account_id: u64 = account_id.into();
        let key_pair = key_pair.to_bytes();
        self.db
            .execute(
                "INSERT INTO account_keys (account_id, key_pair) VALUES (?, ?)",
                params![account_id as i64, key_pair],
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

#[cfg(test)]
mod tests {
    use std::fs;

    use ctor::dtor;

    use rusqlite::{params, Connection};

    use super::{migrations, Store};

    const DB_NAME: &str = "test_db.sqlite3";

    pub fn store_for_tests() -> Store {
        let mut db = Connection::open(DB_NAME).unwrap();
        migrations::update_to_latest(&mut db).unwrap();

        Store { db }
    }

    #[test]
    pub fn insert_u64_max_as_id() {
        let store = store_for_tests();
        let test_value: u64 = u64::MAX;

        store.db.execute(
            "INSERT INTO accounts (id, code_root, storage_root, vault_root, nonce, committed) VALUES (?, '1', '1', '1', '1', '1')",
            params![test_value as i64],
        )
        .unwrap();

        let mut stmt = store.db.prepare("SELECT id from accounts").unwrap();

        let mut rows = stmt.query([]).unwrap();
        while let Some(r) = rows.next().unwrap() {
            let v: i64 = r.get(0).unwrap();
            if v as u64 == test_value {
                return;
            };
        }
        panic!()
    }

    #[dtor]
    fn cleanup() {
        fs::remove_file(DB_NAME).unwrap()
    }
}
