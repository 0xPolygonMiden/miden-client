# Changelog

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
* Add offchain account support for the tonic client method `get_account_update`.
* Refactorized `get_account` to create the account from a single query.
* Admit partial account IDs for the commands that need them.
* Added nextest to be used as test runner.
* Added config file to run integration tests against a remote node.
* Added `CONTRIBUTING.MD` file.
* Renamed `format` command from `Makefile.toml` to `check-format` and added a new `format` command that applies the formatting.
* Added methods to get output notes from client.
* Added a `input-notes list-consumable` command to the CLI.

## 0.2.0 (2024-04-14)

* Added an `init` command to the CLI.
* Added support for on-chain accounts.
* Added support for public notes.
* Added `NoteScreener` struct capable of detecting notes consumable by a client (via heuristics), for storing only relevant notes.
* Added `TransactionRequest` for defining transactions with arbitrary scripts, inputs and outputs and changed the client API to use this definition.
* Added `ClientRng` trait for randomness component within `Client`.
* Refactored integration tests to be ran as regular rust tests.
* Normalized note script fields for input note and output note tables in SQLite implementation.
* Added support for P2IDR (pay-to-id with recall) transactions on both the CLI and the lib.
* Removed the `mock-data` command from the CLI.

## 0.1.0 (2024-03-15)

* Initial release.
