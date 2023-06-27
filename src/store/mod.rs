use super::{errors::StoreError, AccountStub, ClientConfig};
use crypto::{utils::collections::BTreeMap, Word};
use objects::{
    accounts::{Account, AccountCode, AccountStorage, AccountVault},
    assembly::AstSerdeOptions,
    assets::Asset,
};
use rusqlite::{params, Connection, Transaction};

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

    pub fn insert_account_with_metadata(&mut self, account: &Account) -> Result<(), StoreError> {
        let tx = self.db.transaction().unwrap();

        Self::insert_account_code(&tx, account.code())?;
        Self::insert_account_storage(&tx, account.storage())?;
        Self::insert_account_vault(&tx, account.vault())?;
        Self::insert_account(&tx, account)?;

        tx.commit().map_err(StoreError::QueryError)
    }

    fn insert_account(tx: &Transaction<'_>, account: &Account) -> Result<(), StoreError> {
        let id: u64 = account.id().into();
        let code_root = serde_json::to_string(&account.code().root())
            .map_err(StoreError::InputSerializationError)?;
        let storage_root = serde_json::to_string(&account.storage().root())
            .map_err(StoreError::InputSerializationError)?;
        let vault_root = serde_json::to_string(&account.vault().commitment())
            .map_err(StoreError::InputSerializationError)?;

        tx.execute(
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

    fn insert_account_code(
        tx: &Transaction<'_>,
        account_code: &AccountCode,
    ) -> Result<(), StoreError> {
        let code_root = serde_json::to_string(&account_code.root())
            .map_err(StoreError::InputSerializationError)?;
        let code = serde_json::to_string(account_code.procedures())
            .map_err(StoreError::InputSerializationError)?;
        let module = account_code.module().to_bytes(AstSerdeOptions {
            serialize_imports: true,
        });

        tx.execute(
            "INSERT OR IGNORE INTO account_code (root, procedures, module) VALUES (?, ?, ?)",
            params![code_root, code, module,],
        )
        .map(|_| ())
        .map_err(StoreError::QueryError)
    }

    fn insert_account_storage(
        tx: &Transaction<'_>,
        account_storage: &AccountStorage,
    ) -> Result<(), StoreError> {
        let storage_root = serde_json::to_string(&account_storage.root())
            .map_err(StoreError::InputSerializationError)?;

        let storage_slots: BTreeMap<u64, &Word> = account_storage.slots().leaves().collect();
        let storage_slots =
            serde_json::to_string(&storage_slots).map_err(StoreError::InputSerializationError)?;

        tx.execute(
            "INSERT INTO account_storage (root, slots) VALUES (?, ?)",
            params![storage_root, storage_slots],
        )
        .map(|_| ())
        .map_err(StoreError::QueryError)
    }

    fn insert_account_vault(
        tx: &Transaction<'_>,
        account_vault: &AccountVault,
    ) -> Result<(), StoreError> {
        let vault_root = serde_json::to_string(&account_vault.commitment())
            .map_err(StoreError::InputSerializationError)?;

        let assets: Vec<Asset> = account_vault.assets().collect();
        let assets = serde_json::to_string(&assets).map_err(StoreError::InputSerializationError)?;

        tx.execute(
            "INSERT INTO account_vaults (root, assets) VALUES (?, ?)",
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

#[cfg(test)]
mod tests {
    use std::env::temp_dir;
    use uuid::Uuid;

    use crypto::dsa::rpo_falcon512::KeyPair;

    use miden_lib::{assembler::assembler, AuthScheme};
    use objects::{
        accounts::{Account, AccountCode, AccountType},
        assembly::ModuleAst,
    };
    use rusqlite::{params, Connection};

    use crate::store;

    use super::{migrations, Store};

    pub fn store_for_tests() -> Store {
        let mut temp_file = temp_dir();
        temp_file.push(format!("{}.sqlite3", Uuid::new_v4()));
        let mut db = Connection::open(temp_file).unwrap();
        migrations::update_to_latest(&mut db).unwrap();

        Store { db }
    }

    fn test_account_code() -> AccountCode {
        let auth_scheme_procedure = "basic::auth_tx_rpo_falcon512";

        let account_code_string: String = format!(
            "
    use.miden::wallets::basic->basic_wallet
    use.miden::eoa::basic

    export.basic_wallet::receive_asset
    export.basic_wallet::send_asset
    export.{auth_scheme_procedure}

    "
        );
        let account_code_src: &str = &account_code_string;
        let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
        let account_assembler = assembler();
        AccountCode::new(account_code_ast.clone(), &account_assembler).unwrap()
    }

    fn test_account() -> Account {
        let init_seed = [0u8; 32];
        let key_pair: KeyPair = KeyPair::new().unwrap();
        let auth_scheme = AuthScheme::RpoFalcon512 {
            pub_key: key_pair.public_key(),
        };
        let (acc, _) = miden_lib::wallets::create_basic_wallet(
            init_seed,
            auth_scheme,
            AccountType::RegularAccountImmutableCode,
        )
        .unwrap();
        acc
    }

    #[test]
    pub fn test_insert_u64_max_as_id() {
        let store = store_for_tests();
        let test_value: u64 = u64::MAX;

        // Insert dummy data on tables to prevent foreing key constraint errors
        store
            .db
            .execute(
                "INSERT INTO account_code (root, procedures, module) VALUES ('1', '1', '1')",
                [],
            )
            .unwrap();

        store
            .db
            .execute(
                "INSERT INTO account_storage (root, slots) VALUES ('1', '1')",
                [],
            )
            .unwrap();

        store
            .db
            .execute(
                "INSERT INTO account_vaults (root, assets) VALUES ('1', '1')",
                [],
            )
            .unwrap();

        // Actual test
        store.db.execute(
            "INSERT INTO accounts (id, code_root, storage_root, vault_root, nonce, committed) VALUES (?, '1', '1', '1', '1', '1')",
            params![test_value as i64],
        )
        .unwrap();

        let actual: i64 = store
            .db
            .query_row("SELECT id from accounts", [], |row| row.get(0))
            .unwrap();

        assert_eq!(actual as u64, test_value);
    }

    #[test]
    pub fn insert_same_account_twice_fails() {
        let mut store = store_for_tests();
        let account = test_account();

        assert!(store.insert_account_with_metadata(&account).is_ok());
        assert!(store.insert_account_with_metadata(&account).is_err());
    }

    #[test]
    fn test_account_code_insertion_no_duplicates() {
        let mut store = store_for_tests();
        let account_code = test_account_code();
        let tx = store.db.transaction().unwrap();

        // Table is empty at the beginning
        let mut actual: usize = tx
            .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, 0);

        // First insertion generates a new row
        store::Store::insert_account_code(&tx, &account_code).unwrap();
        actual = tx
            .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, 1);

        // Second insertion does not generate a new row
        store::Store::insert_account_code(&tx, &account_code).unwrap();
        actual = tx
            .query_row("SELECT Count(*) FROM account_code", [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, 1);
    }
}
