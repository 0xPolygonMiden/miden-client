# Rust Client Library

Rust library, which can be used by other project to programmatically interact with the Miden rollup.

## Adding miden-client as a dependency

In order to utilize the `miden-client` library, you can add the dependency to your project's `Cargo.toml` file:

````toml
miden-client = { version = "0.9" }
````

## Crate Features

| Features     | Description                                                                                                                                               |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `concurrent` | Used to enable concurrency during execution and proof generation. **Disabled by default.**                                                               |
| `idxdb`      | Includes `WebStore`, an IndexedDB implementation of the `Store` trait. **Disabled by default.**                                                          |
| `sqlite`     | Includes `SqliteStore`, a SQLite implementation of the `Store` trait. This relies on the standard library. **Disabled by default.**                                                           |
| `tonic`      | Includes `TonicRpcClient`, a `std`-compatible Tonic client to communicate with Miden node. This relies on the `tonic` for the inner transport.  **Disabled by default.**                                                        |
| `web-tonic`  | Includes `TonicRpcClient`, a `wasm`-compatible Tonic client to communicate with the Miden node. This relies on `tonic-web-wasm-client` for the inner transport. **Disabled by default.**                                   |
| `testing`    | Enables functions meant to be used in testing environments. **Disabled by default.**             |

Features `sqlite` and `idxdb` are mutually exclusive, the same goes for `tonic` and `web-tonic`.

### Store and RpcClient implementations

The library user can provide their own implementations of `Store` and `RpcClient` traits, which can be used as components of `Client`, though it is not necessary. The `Store` trait is used to persist the state of the client, while the `RpcClient` trait is used to communicate via [gRPC](https://grpc.io/) with the Miden node.

The `sqlite` and `tonic` features provide implementations for these traits using [Rusqlite](https://github.com/rusqlite/rusqlite) and [Tonic](https://github.com/hyperium/tonic) respectively. The `idxdb` and `web-tonic` features provide implementations based on [IndexedDB](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API) and [tonic-web](https://github.com/hyperium/tonic/tree/master/tonic-web) which can be used in the browser.

## License
This project is [MIT licensed](../../LICENSE).
