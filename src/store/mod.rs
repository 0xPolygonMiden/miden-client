use super::{errors::StoreError, AccountStub, ClientConfig};
use crypto::{hash::rpo::Rpo256, Felt};
use rusqlite::Connection;

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
        migrations::insert_mock_data(&db);

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
            let id: u64 = row.get(0).unwrap();
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
