# Changelog

* Added `TransactionRequest` for defining transactions with arbitrary scripts, inputs and outputs and changed the client API to use this definition.
* Added `ClientRng` trait for randomness component within `Client`.
* Refactored integration tests to be ran as regular rust tests.
* Normalized note script fields for input note and output note tables in SQLite implementation.
* Added support for P2IDR (Pay To ID with recall) transactions on both the CLI and the lib.
* Removed `MockDataStore` and replaced usage in affected tests for loading mock data on the DB.
* Removed the `mock-data` command from the CLI.
* Removed `MockClient` altogether from the CLI.

## 0.1.0 (2024-03-15)

* Initial release.
