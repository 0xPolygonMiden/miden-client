# Rust Client Library

Rust library, which can be used by other project to programmatically interact with the Miden rollup.

## Adding miden-client as a dependency

In order to utilize the `miden-client` library, you can add the dependency to your project's `Cargo.toml` file:

````toml
miden-client = { version = "0.3.1" }
````

## Crate Features

- `concurrent`: used to enable concurrency during execution and proof generation.
- `testing`: useful feature that lowers PoW difficulty when enabled, meant to be used during development and not on production.
- `sqlite`: includes `SqliteStore`, a SQLite implementation of the `Store` trait that can be used as a component of `Client`.
- `async`: enables async traits. Disabled by default.
- `tonic`: includes `TonicRpcClient`, a Tonic client to communicate with Miden node, that can be used as a component of `Client`.

To compile with `no_std`, disable default features via `--no-default-features` flag.

### Store and RpcClient implementations

The library user can provide their own implementations of `Store` and `RpcClient` traits, which can be used as components of `Client`, though it is not necessary. The `Store` trait is used to persist the state of the client, while the `RpcClient` trait is used to communicate via [gRPC](https://grpc.io/) with the Miden node.

The `sqlite` and `tonic` features provide implementations for these traits using [Rusqlite](https://github.com/rusqlite/rusqlite) and [Tonic](https://github.com/hyperium/tonic) respectively.

## License
This project is [MIT licensed](../../LICENSE).
