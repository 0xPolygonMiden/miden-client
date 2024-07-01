# Rust Client Library

Rust library, which can be used by other project to programmatically interact with the Miden rollup.

## Adding miden-client as a dependency

In order to utilize the `miden-client` library, you can add the dependency to your project's `Cargo.toml` file:

````toml
miden-client = { version = "0.3.1" }
````

By default, the library is `no_std` compatible.

### Features

- `concurrent`: used to enable concurrency during execution and proof generation.
- `testing`: useful feature that lowers PoW difficulty when enabled, meant to be used during development and not on production.
- `sqlite`: includes `SqliteStore`, a SQLite implementation of the `Store` trait that can be used as a component of `Client`.
- `async`: enables async traits. Disabled by default.
- `tonic`: includes `TonicRpcClient`, a Tonic client to communicate with Miden node, that can be used as a component of `Client`.
- `executable`: builds the CLI, based on `SqliteStore` and `TonicRpcClient`.
