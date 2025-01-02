# Miden remote transaction prover

This crate provides a `RemoteTransactionProver`, a client struct that can be used to interact with the prover service from a Rust codebase.

## Features

Description of this crate's feature:

| Features     | Description                                                                                                 |
| ------------ | ------------------------------------------------------------------------------------------------------------|
| `std`        | Enable usage of Rust's `std`, use `--no-default-features` for `no-std` support.                             |
| `testing`    | Enables testing utilities and reduces proof-of-work requirements to speed up tests' runtimes.               |
