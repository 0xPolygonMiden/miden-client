# Changelog

* Adds `TransactionRequest` for defining transactions with arbitrary scripts, inputs and outputs
* Adds support for P2IDR (Pay To ID with Recall) transactions on both the CLI
  and the lib.
* Removed `MockDataStore` and replaced usage in affected tests for loading mock
  data on the DB.
* Removed the `mock-data` command from the CLI
* Removed `MockClient` altogether from the CLI

## 0.1.0 (2024-03-15)

* Initial release.
