use objects::{
    accounts::{Account, AccountId, AccountStub},
    notes::RecordedNote,
    Digest,
};
use std::path::PathBuf;

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

    // ACCOUNT INSERTION
    // --------------------------------------------------------------------------------------------

    /// Inserts a new account into the client's store.
    pub fn insert_account_with_metadata(&mut self, account: &Account) -> Result<(), ClientError> {
        self.store
            .insert_account_with_metadata(account)
            .map_err(ClientError::StoreError)
    }

    // ACCOUNT DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns summary info about the accounts managed by this client.
    ///
    /// TODO: replace `AccountStub` with a more relevant structure.
    pub fn get_accounts(&self) -> Result<Vec<AccountStub>, ClientError> {
        self.store.get_accounts().map_err(|err| err.into())
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

    // INPUT NOTE DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns input notes managed by this client.
    pub fn get_input_notes(&self) -> Result<Vec<RecordedNote>, ClientError> {
        self.store.get_input_notes().map_err(|err| err.into())
    }

    /// Returns the input note with the specified hash.
    pub fn get_input_note(&self, hash: Digest) -> Result<RecordedNote, ClientError> {
        self.store
            .get_input_note_by_hash(hash)
            .map_err(|err| err.into())
    }

    // INPUT NOTE CREATION
    // --------------------------------------------------------------------------------------------

    /// Inserts a new input note into the client's store.
    pub fn insert_input_note(&mut self, note: RecordedNote) -> Result<(), ClientError> {
        self.store
            .insert_input_note(&note)
            .map_err(|err| err.into())
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

impl ClientConfig {
    /// Returns a new instance of [ClientConfig] with the specified store path and node endpoint.
    pub fn new(store_path: String, node_endpoint: Endpoint) -> Self {
        Self {
            store_path,
            node_endpoint,
        }
    }
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

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::store::tests::create_test_store_path;
    use mock::mock::{
        account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs,
    };

    #[test]
    fn test_input_notes_round_trip() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = super::Client::new(super::ClientConfig::new(
            store_path.into_os_string().into_string().unwrap(),
            super::Endpoint::default(),
        ))
        .unwrap();

        // generate test data
        let (_, _, _, recorded_notes) = mock_inputs(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
        );

        // insert notes into database
        for note in recorded_notes.iter().cloned() {
            client.insert_input_note(note).unwrap();
        }

        // retrieve notes from database
        let retrieved_notes = client.get_input_notes().unwrap();

        // compare notes
        assert_eq!(recorded_notes, retrieved_notes);
    }

    #[test]
    fn test_get_input_note() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = super::Client::new(super::ClientConfig::new(
            store_path.into_os_string().into_string().unwrap(),
            super::Endpoint::default(),
        ))
        .unwrap();

        // generate test data
        let (_, _, _, recorded_notes) = mock_inputs(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
        );

        // insert note into database
        client.insert_input_note(recorded_notes[0].clone()).unwrap();

        // retrieve note from database
        let retrieved_note = client
            .get_input_note(recorded_notes[0].note().hash())
            .unwrap();

        // compare notes
        assert_eq!(recorded_notes[0], retrieved_note);
    }
}
