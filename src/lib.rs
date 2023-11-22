use crypto::{
    dsa::rpo_falcon512::KeyPair, hash::rpo::RpoDigest, utils::collections::BTreeMap, Word,
};
use objects::{
    accounts::{Account, AccountId, AccountStub},
    assembly::ModuleAst,
    assets::Asset,
};
use std::{path::PathBuf, str::FromStr};

mod store;
use store::Store;

pub mod errors;
use errors::ClientError;

// MIDEN CLIENT
// ================================================================================================

/// A light client for connecting to the Miden rollup network.
///
/// Miden client is responsible for managing a set of accounts. Specifically, the client:
/// - Keeps track of the current and historical states of a set of accounts and related objects
///   such as notes and transactions.
/// - Connects to one or more Miden nodes to periodically sync with the current state of the
///   network.
/// - Executes, proves, and submits transactions to the network as directed by the user.
pub struct Client {
    /// Local database containing information about the accounts managed by this client.
    store: Store,
    // TODO
    // node: connection to Miden node
}

impl Client {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of [Client] instantiated with the specified configuration options.
    ///
    /// # Errors
    /// Returns an error if the client could not be instantiated.
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        Ok(Self {
            store: Store::new((&config).into())?,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the store
    pub fn store(&self) -> &Store {
        &self.store
    }

    // DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns summary info about the accounts managed by this client.
    ///
    /// TODO: replace `AccountStub` with a more relevant structure.
    pub fn get_accounts(&self) -> Result<Vec<AccountStub>, ClientError> {
        self.store.get_accounts().map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    pub fn get_account_by_id(&self, account_id: AccountId) -> Result<AccountStub, ClientError> {
        self.store
            .get_account_by_id(account_id)
            .map_err(|err| err.into())
    }

    /// Returns key pair structure for an Account Id.
    pub fn get_account_keys(&self, account_id: AccountId) -> Result<KeyPair, ClientError> {
        self.store
            .get_account_keys(account_id)
            .map_err(|err| err.into())
    }

    /// Returns vault assets from a vault root.
    pub fn get_vault_assets(&self, vault_root: RpoDigest) -> Result<Vec<Asset>, ClientError> {
        self.store
            .get_vault_assets(vault_root)
            .map_err(|err| err.into())
    }

    /// Returns account code data from a root.
    pub fn get_account_code(
        &self,
        code_root: RpoDigest,
    ) -> Result<(Vec<RpoDigest>, ModuleAst), ClientError> {
        self.store
            .get_account_code(code_root)
            .map_err(|err| err.into())
    }

    /// Returns account storage data from a storage root.
    pub fn get_account_storage(
        &self,
        storage_root: RpoDigest,
    ) -> Result<BTreeMap<u64, Word>, ClientError> {
        self.store
            .get_account_storage(storage_root)
            .map_err(|err| err.into())
    }

    /// Returns historical states for the account with the specified ID.
    ///
    /// TODO: wrap `Account` in a type with additional info.
    /// TODO: consider changing the interface to support pagination.
    pub fn get_account_history(&self, _account_id: AccountId) -> Result<Vec<Account>, ClientError> {
        todo!()
    }

    /// Returns detailed information about the current state of the account with the specified ID.
    ///
    /// TODO: wrap `Account` in a type with additional info (e.g., status).
    /// TODO: consider adding `nonce` as another parameter to identify a specific account state.
    pub fn get_account_details(&self, _account_id: AccountId) -> Result<Account, ClientError> {
        todo!()
    }

    // TODO: add methods for retrieving note and transaction info, and for creating/executing
    // transaction
}

// CLIENT CONFIG
// ================================================================================================

/// Configuration options of Miden client.
#[derive(Debug, PartialEq, Eq)]
pub struct ClientConfig {
    /// Location of the client's data file.
    store_path: String,
    /// Address of the Miden node to connect to.
    node_endpoint: Endpoint,
}

impl Default for ClientConfig {
    fn default() -> Self {
        const STORE_FILENAME: &str = "store.sqlite3";

        // get directory of the currently executing binary, or fallback to the current directory
        let exec_dir = match std::env::current_exe() {
            Ok(mut path) => {
                path.pop();
                path
            }
            Err(_) => PathBuf::new(),
        };

        let store_path = exec_dir.join(STORE_FILENAME);

        Self {
            store_path: store_path
                .into_os_string()
                .into_string()
                .expect("Creating the hardcoded path to the store file should not panic"),
            node_endpoint: Endpoint::default(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

impl Default for Endpoint {
    fn default() -> Self {
        const MIDEN_NODE_PORT: u16 = 57291;

        Self {
            host: "localhost".to_string(),
            port: MIDEN_NODE_PORT,
        }
    }
}

/// Directory to store Miden-related files (e.g. defaults, databases). Typically ~/.miden/
pub fn miden_home() -> PathBuf {
    std::env::var("MIDEN_HOME")
        .map(|path| PathBuf::from_str(&path).unwrap_or_default())
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".miden"))
}
