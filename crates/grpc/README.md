# Miden Client gRPC

Default gRPC implementation of the `NodeRpcClient` trait from `miden-client-core`.

- `GrpcClient`, which implements `NodeRpcClient` and talks to a Miden node over gRPC
- The protobuf code generated from the node RPC API, and the conversions between its messages and the `miden-client-core` types
- The parsing of the typed errors that the node returns

On native targets, `GrpcClient` uses the `tonic` transport with TLS. On `wasm32` targets, it uses `tonic-web-wasm-client`.

## Quick Start

Add to `Cargo.toml`:

```toml
miden-client-core = { version = "0.17" }
miden-client-grpc = { version = "0.17" }
```

To build for `wasm32`, turn off the default features:

```toml
miden-client-grpc = { version = "0.17", default-features = false }
```

## Usage

```rust
use miden_client_core::rpc::{Endpoint, NodeRpcClient};
use miden_client_grpc::GrpcClient;

let client = GrpcClient::new(&Endpoint::testnet(), 10_000);
let (block_header, _) = client.get_block_header_by_number(None, false).await?;
```

## Features

| Feature | Description                                                                        |
|---------|------------------------------------------------------------------------------------|
| `std`   | Enables the Rust standard library and the native `tonic` transport with TLS. On by default. |

## License
This project is licensed under the MIT License. See the [LICENSE](../../LICENSE) file for details.
