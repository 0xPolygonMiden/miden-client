use super::Store;

use super::errors::StoreError;
use super::AccountStub;
use crypto::hash::rpo::RpoDigest;
use crypto::{
    dsa::rpo_falcon512::KeyPair,
    utils::{collections::BTreeMap, Deserializable, Serializable},
    Word,
};
use objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault},
    assembly::{AstSerdeOptions, ModuleAst},
    assets::Asset,
    Digest,
};
use rusqlite::{params, Transaction};

impl Store {
    // ACCOUNTS
    // --------------------------------------------------------------------------------------------

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
                serde_json::from_str(&vault_root)
                    .map_err(StoreError::JsonDataDeserializationError)?,
                serde_json::from_str(&storage_root)
                    .map_err(StoreError::JsonDataDeserializationError)?,
                serde_json::from_str(&code_root)
                    .map_err(StoreError::JsonDataDeserializationError)?,
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
        let account_id_int: u64 = account_id.into();

        let mut rows = stmt
            .query(params![account_id_int as i64])
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
                serde_json::from_str(&vault_root)
                    .map_err(StoreError::JsonDataDeserializationError)?,
                serde_json::from_str(&storage_root)
                    .map_err(StoreError::JsonDataDeserializationError)?,
                serde_json::from_str(&code_root)
                    .map_err(StoreError::JsonDataDeserializationError)?,
            );

            Ok(account)
        } else {
            Err(StoreError::AccountDataNotFound(account_id))
        }
    }

    /// Retrieve account keys data by Account Id
    pub fn get_account_auth(&self, account_id: AccountId) -> Result<AuthInfo, StoreError> {
        let mut stmt = self
            .db
            .prepare("SELECT auth_info FROM account_auth WHERE account_id = ?")
            .map_err(StoreError::QueryError)?;
        let account_id_int: u64 = account_id.into();

        let mut rows = stmt
            .query(params![account_id_int as i64])
            .map_err(StoreError::QueryError)?;

        if let Some(row) = rows.next().map_err(StoreError::QueryError)? {
            let auth_info_bytes: Vec<u8> = row.get(0).map_err(StoreError::QueryError)?;
            let auth_info: AuthInfo = AuthInfo::read_from_bytes(&auth_info_bytes)
                .map_err(StoreError::DataDeserializationError)?;
            Ok(auth_info)
        } else {
            Err(StoreError::AccountDataNotFound(account_id))
        }
    }

    /// Retrieve account code-related data by code root
    pub fn get_account_code(
        &self,
        root: Digest,
    ) -> Result<(Vec<RpoDigest>, ModuleAst), StoreError> {
        let root_serialized =
            serde_json::to_string(&root).map_err(StoreError::InputSerializationError)?;

        let mut stmt = self
            .db
            .prepare("SELECT procedures, module FROM account_code WHERE root = ?")
            .map_err(StoreError::QueryError)?;
        let mut rows = stmt
            .query(params![root_serialized])
            .map_err(StoreError::QueryError)?;

        if let Some(row) = rows.next().map_err(StoreError::QueryError)? {
            let procedures: String = row.get(0).map_err(StoreError::QueryError)?;
            let module: Vec<u8> = row.get(1).map_err(StoreError::QueryError)?;

            let procedures = serde_json::from_str(&procedures)
                .map_err(StoreError::JsonDataDeserializationError)?;
            let module =
                ModuleAst::from_bytes(&module).map_err(StoreError::DataDeserializationError)?;
            Ok((procedures, module))
        } else {
            Err(StoreError::AccountCodeDataNotFound(root))
        }
    }

    /// Retrieve account storage data by vault root
    pub fn get_account_storage(&self, root: RpoDigest) -> Result<BTreeMap<u64, Word>, StoreError> {
        let root_serialized =
            serde_json::to_string(&root).map_err(StoreError::InputSerializationError)?;

        let mut stmt = self
            .db
            .prepare("SELECT slots FROM account_storage WHERE root = ?")
            .map_err(StoreError::QueryError)?;
        let mut rows = stmt
            .query(params![root_serialized])
            .map_err(StoreError::QueryError)?;

        if let Some(row) = rows.next().map_err(StoreError::QueryError)? {
            let slots: String = row.get(0).map_err(StoreError::QueryError)?;
            let slots =
                serde_json::from_str(&slots).map_err(StoreError::JsonDataDeserializationError)?;
            Ok(slots)
        } else {
            Err(StoreError::AccountStorageNotFound(root))
        }
    }

    /// Retrieve assets by vault root
    pub fn get_vault_assets(&self, root: RpoDigest) -> Result<Vec<Asset>, StoreError> {
        let vault_root =
            serde_json::to_string(&root).map_err(StoreError::InputSerializationError)?;

        let mut stmt = self
            .db
            .prepare("SELECT assets FROM account_vaults WHERE root = ?")
            .map_err(StoreError::QueryError)?;
        let mut rows = stmt
            .query(params![vault_root])
            .map_err(StoreError::QueryError)?;

        if let Some(row) = rows.next().map_err(StoreError::QueryError)? {
            let assets: String = row.get(0).map_err(StoreError::QueryError)?;
            let assets =
                serde_json::from_str(&assets).map_err(StoreError::JsonDataDeserializationError)?;
            Ok(assets)
        } else {
            Err(StoreError::VaultDataNotFound(root))
        }
    }

    pub fn insert_account(
        &mut self,
        account: &Account,
        key_pair: &KeyPair,
    ) -> Result<(), StoreError> {
        let tx = self
            .db
            .transaction()
            .map_err(StoreError::TransactionError)?;

        Self::insert_account_code(&tx, account.code())?;
        Self::insert_account_storage(&tx, account.storage())?;
        Self::insert_account_vault(&tx, account.vault())?;
        Self::insert_account_record(&tx, account)?;
        Self::insert_account_auth(&tx, account.id(), key_pair)?;

        tx.commit().map_err(StoreError::TransactionError)
    }

    fn insert_account_record(tx: &Transaction<'_>, account: &Account) -> Result<(), StoreError> {
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

    pub fn insert_account_auth(
        tx: &Transaction<'_>,
        account_id: AccountId,
        key_pair: &KeyPair,
    ) -> Result<(), StoreError> {
        let account_id: u64 = account_id.into();
        let auth_info = AuthInfo::RpoFalcon512(*key_pair).to_bytes();
        tx.execute(
            "INSERT INTO account_auth (account_id, auth_info) VALUES (?, ?)",
            params![account_id as i64, auth_info],
        )
        .map(|_| ())
        .map_err(StoreError::QueryError)
    }
}

// DATABASE AUTH INFO
// ================================================================================================

/// Type of Authentication Methods supported by the DB
///
/// TODO: add remaining auth types
pub enum AuthInfo {
    RpoFalcon512(KeyPair),
}

const RPO_FALCON512_AUTH: u8 = 0;

impl AuthInfo {
    /// Returns byte identifier of specific AuthInfo
    fn type_byte(&self) -> u8 {
        match self {
            AuthInfo::RpoFalcon512(_) => RPO_FALCON512_AUTH,
        }
    }
}

impl Serializable for AuthInfo {
    fn write_into<W: crypto::utils::ByteWriter>(&self, target: &mut W) {
        let mut bytes = vec![self.type_byte()];
        match self {
            AuthInfo::RpoFalcon512(key_pair) => {
                bytes.append(&mut key_pair.to_bytes());
                target.write_bytes(&bytes);
            }
        }
    }
}

impl Deserializable for AuthInfo {
    fn read_from<R: crypto::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, crypto::utils::DeserializationError> {
        let auth_type: u8 = source.read_u8()?;
        match auth_type {
            RPO_FALCON512_AUTH => {
                let key_pair = KeyPair::read_from(source)?;
                Ok(AuthInfo::RpoFalcon512(key_pair))
            }
            val => Err(crypto::utils::DeserializationError::InvalidValue(
                val.to_string(),
            )),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::AuthInfo;
    use crypto::{
        dsa::rpo_falcon512::KeyPair,
        utils::{Deserializable, Serializable},
    };
    use miden_lib::assembler::assembler;
    use mock::mock::account;
    use rusqlite::Connection;

    use crate::store::{self, migrations, tests::create_test_store_path};

    use super::Store;

    fn create_test_store() -> Store {
        let temp_file = create_test_store_path();
        let mut db = Connection::open(temp_file).unwrap();
        migrations::update_to_latest(&mut db).unwrap();

        Store { db }
    }
    #[test]
    fn test_auth_info_serialization() {
        let exp_key_pair = KeyPair::new().unwrap();
        let auth_info = AuthInfo::RpoFalcon512(exp_key_pair);
        let bytes = auth_info.to_bytes();
        let actual = AuthInfo::read_from_bytes(&bytes).unwrap();
        match actual {
            AuthInfo::RpoFalcon512(act_key_pair) => {
                assert_eq!(exp_key_pair, act_key_pair)
            }
        }
    }

    #[test]
    fn test_account_code_insertion_no_duplicates() {
        let mut store = create_test_store();
        let assembler = assembler();
        let account_code = account::mock_account_code(&assembler);
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
