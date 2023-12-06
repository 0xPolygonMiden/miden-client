use crate::store::Store;

use self::{config::ClientConfig, errors::ClientError};

pub mod accounts;
pub mod config;
pub mod errors;
pub mod notes;

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
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use crate::client::{config::Endpoint, Client};

    use crate::client::config::ClientConfig;
    use crate::store::tests::create_test_store_path;
    use crypto::dsa::rpo_falcon512::KeyPair;
    use miden_lib::assembler::assembler;
    use mock::mock::{
        account::{self, MockAccountType},
        notes::AssetPreservationStatus,
        transaction::mock_inputs,
    };

    #[test]
    fn test_input_notes_round_trip() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = Client::new(ClientConfig::new(
            store_path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
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
        let mut client = Client::new(ClientConfig::new(
            store_path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
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

    #[test]
    pub fn insert_same_account_twice_fails() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = Client::new(ClientConfig::new(
            store_path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
        ))
        .unwrap();

        let assembler = assembler();
        let account = account::mock_new_account(&assembler);

        let key_pair: KeyPair = KeyPair::new()
            .map_err(|err| format!("Error generating KeyPair: {}", err))
            .unwrap();

        assert!(client.insert_account(&account, &key_pair).is_ok());
        assert!(client.insert_account(&account, &key_pair).is_err());
    }
}
