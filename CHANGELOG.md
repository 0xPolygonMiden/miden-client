# Changelog

<<<<<<< HEAD
* Added `CONTRIBUTING.MD` file.
* Renamed `format` command from `Makefile.toml` to `check-format` and added a
  new `format` command that applies the formatting.
||||||| 54427b1
=======
* Refactored integration tests to be ran as regular rust tests.
* Normalize note script fields for input note and output note tables in sqlite
  implementation.
>>>>>>> 7bceaedf5ab5af22073fcc66058200be0153f2e4
* Adds support for P2IDR (Pay To ID with Recall) transactions on both the CLI
  and the lib.
* Removed `MockDataStore` and replaced usage in affected tests for loading mock
  data on the DB.
* Removed the `mock-data` command from the CLI
* Removed `MockClient` altogether from the CLI

## 0.1.0 (2024-03-15)

* Initial release.
