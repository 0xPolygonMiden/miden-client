# Changelog

## 0.6.0 (TBD)

* [BREAKING] Refactored Client struct to use trait objects for inner struct fields (#539).
* Fixed panic on export command without type (#537).
* Added Account Integration Tests for Web Client (#532).
* [BREAKING] Refactored the `Store` structure and interface for input notes (#520).
* Fixed Broken WASM (#519).
* [BREAKING] Changed `PaymentTransactionData` and `TransactionRequest` to allow for multiple assets per note (#525).
* [BREAKING] Removed serde's de/serialization from `NoteRecordDetails` and `NoteStatus` (#514).
* Added support for custom transactions in web client (#519).
* [BREAKING] Renamed `off-chain` and `on-chain` to `private` and `public` respectively for the account storage modes (#516).
* [BREAKING] Added IDs to `SyncSummary` fields (#513).
* [BREAKING] Re-exported `TransactionRequest` from submodule, renamed `AccountDetails::Offchain` to `AccountDetails::Private`, renamed `NoteDetails::OffChain` to `NoteDetails::Private` (#508).

## v0.5.0 (2024-08-27)

### Features

* Added support for decimal values in the CLI (#454).
* Added serialization for `TransactionRequest` (#471).
* Added support for importing committed notes from older blocks than current (#472).
* Added support for account export in the CLI (#479).
* Added the Web Client Crate (#437)
* Added testing suite for the Web Client Crate (#498)
* Fixed typing for the Web Client Crate (#521)
* [BREAKING] Refactored `TransactionRequest` to represent a generalized transaction (#438).

### Enhancements

* Added conversions for `NoteRecordDetails` (#392).
* Ignored stale updates received during sync process (#412).
* Changed `TransactionRequest` to use `AdviceInputs` instead of `AdviceMap` (#436).
* Tracked token symbols with config file (#441).
* Added validations in transaction requests (#447).
* [BREAKING] Track expected block height for notes (#448).
* Added validation for consumed notes when importing (#449).
* [BREAKING] Removed `TransactionTemplate` and `account_id` from `TransactionRequest` (#478).

### Changes

* Refactor `TransactionRequest` constructor (#434).
* [BREAKING] Refactored `Client` to merge submit_transaction and prove_transaction (#445).
* Change schema and code to to reflect changes to `NoteOrigin` (#463).
* [BREAKING] Updated Rust Client to use the new version of `miden-base` (#492).

### Fixes

* Fixed flaky integration tests (#410).
* Fixed `get_consumable_notes` to consider block header information for consumability (#432).

## v0.4.1 (2024-07-08) - `miden-client` crete only

* Fixed the build script to avoid updating generated files in docs.rs environment (#433).

## v0.4.0 (2024-07-05)

### Features

* [BREAKING] Separated `prove_transaction` from `submit_transaction` in `Client`. (#339)
* Note importing in client now uses the `NoteFile` type (#375).
* Added `wasm` and `async` feature to make the code compatible with WASM-32 target (#378).
* Added WebStore to the miden-client to support WASM-compatible store mechanisms (#401).
* Added WebTonicClient to the miden-client to support WASM-compatible RPC calls (#409).
* [BREAKING] Added unauthenticated notes to `TransactionRequest` and necessary changes to consume unauthenticated notes with the client (#417).
* Added advice map to `TransactionRequest` and updated integration test with example using the advice map to provide more than a single `Word` as `NoteArgs` for a note (#422).
* Made the client `no_std` compatible (#428).

### Enhancements

* Fixed the error message when trying to consume a pending note (now it shows that the transaction is not yet ready to be consumed).
* Added created and consumed note info when printing the transaction summary on the CLI. (#348).
* [BREAKING] Updated CLI commands so assets are now passed as `<AMOUNT>::<FAUCET_ACCOUNT_ID>` (#349).
* Changed `consume-notes` to pick up the default account ID if none is provided, and to consume all notes that are consumable by the ID if no notes are provided to the list. (#350).
* Added integration tests using the CLI (#353).
* Simplified and separated the `notes --list` table (#356).
* Fixed bug when exporting a note into a file (#368).
* Added a new check on account creation / import on the CLI to set the account as the default one if none is set (#372).
* Changed `cargo-make` usage for `make` and `Makefile.toml` for a regular `Makefile` (#359).
* [BREAKING] Library API reorganization (#367).
* New note status added to reflect more possible states (#355).
* Renamed "pending" notes to "expected" notes (#373).
* Implemented retrieval of executed transaction info (id, commit height, account_id) from sync state RPC endpoint (#387).
* Added build script to import Miden node protobuf files to generate types for `tonic_client` and removed `miden-node-proto` dependency (#395).
* [BREAKING] Split cli and client into workspace (#407).
* Moved CLI tests to the `miden-cli` crate (#413).
* Restructured the client crate module organization (#417).

## v0.3.1 (2024-05-22)

* No changes; re-publishing to crates.io to re-build documentation on docs.rs.

## v0.3.0 (2024-05-17)

* Added swap transactions and example flows on integration tests.
* Flatten the CLI subcommand tree.
* Added a mechanism to retrieve MMR data whenever a note created on a past block is imported.
* Changed the way notes are added to the database based on `ExecutedTransaction`.
* Added more feedback information to commands `info`, `notes list`, `notes show`, `account new`, `notes import`, `tx new` and `sync`.
* Add `consumer_account_id` to `InputNoteRecord` with an implementation for sqlite store.
* Renamed the CLI `input-notes` command to `notes`. Now we only export notes that were created on this client as the result of a transaction.
* Added validation using the `NoteScreener` to see if a block has relevant notes.
* Added flags to `init` command for non-interactive environments
* Added an option to verify note existence in the chain before importing.
* Add new store note filter to fetch multiple notes by their id in a single query.
* [BREAKING] `Client::new()` now does not need a `data_store_store` parameter, and `SqliteStore`'s implements interior mutability.
* [BREAKING] The store's `get_input_note` was replaced by `get_input_notes` and a `NoteFilter::Unique` was added.
* Refactored `get_account` to create the account from a single query.
* Added support for using an account as the default for the CLI
* Replace instead of ignore note scripts with when inserting input/output notes with a previously-existing note script root to support adding debug statements.
* Added RPC timeout configuration field
* Add off-chain account support for the tonic client method `get_account_update`.
* Refactored `get_account` to create the account from a single query.
* Admit partial account IDs for the commands that need them.
* Added nextest to be used as test runner.
* Added config file to run integration tests against a remote node.
* Added `CONTRIBUTING.MD` file.
* Renamed `format` command from `Makefile.toml` to `check-format` and added a new `format` command that applies the formatting.
* Added methods to get output notes from client.
* Added a `input-notes list-consumable` command to the CLI.

## 0.2.1 (2024-04-24)

* Added ability to start the client in debug mode (#283).

## 0.2.0 (2024-04-14)

* Added an `init` command to the CLI.
* Added support for on-chain accounts.
* Added support for public notes.
* Added `NoteScreener` struct capable of detecting notes consumable by a client (via heuristics), for storing only relevant notes.
* Added `TransactionRequest` for defining transactions with arbitrary scripts, inputs and outputs and changed the client API to use this definition.
* Added `ClientRng` trait for randomness component within `Client`.
* Refactored integration tests to be run as regular rust tests.
* Normalized note script fields for input note and output note tables in SQLite implementation.
* Added support for P2IDR (pay-to-id with recall) transactions on both the CLI and the lib.
* Removed the `mock-data` command from the CLI.

## 0.1.0 (2024-03-15)

* Initial release.
