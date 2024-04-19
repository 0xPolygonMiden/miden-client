# Miden client

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/0xPolygonMiden/miden-client/blob/main/LICENSE)
[![CI](https://github.com/0xPolygonMiden/miden-client/actions/workflows/ci.yml/badge.svg)](https://github.com/0xPolygonMiden/miden-clinet/actions/workflows/ci.yml)
[![RUST_VERSION](https://img.shields.io/badge/rustc-1.77+-lightgray.svg)]()
[![crates.io](https://img.shields.io/crates/v/miden-client)](https://crates.io/crates/miden-client)

This repository contains the Miden client, which provides a way to execute and prove transactions, facilitating the interaction with the Miden rollup.

### Status

The Miden client is still under heavy development and the project can be considered to be in an *alpha* stage. Many features are yet to be implemented and there is a number of limitations which we will lift in the near future.

## Overview

The Miden client currently consists of two components:

- `miden-client` library, which can be used by other project to programmatically interact with the Miden rollup. 
- `miden-client` binary which is a wrapper around the library exposing its functionality via a simple command-line interface (CLI).

The client's main responsibility is to maintain a partial view of the blockchain which allows for locally executing and proving transactions. It keeps a local store of various entities that periodically get updated by syncing with the node.

## Dependencies

### Required

- [Rust installation](https://www.rust-lang.org/tools/install) with version 1.77 or higher.

### Optional

- [Cargo Make](https://github.com/sagiegurari/cargo-make)

## Usage

### Adding miden-client as a dependency

In order to utilize the `miden-client` library, you can add the dependency to your project's `Cargo.toml` file:

````toml
miden-client = { version = "0.2" }
````

### Running `miden-client`'s CLI

You can either build from source with:

```bash
cargo build --release
```

Once the binary is built, you can find it on `./target/release/miden-client`.

Or you can install the CLI from crates-io with:

```bash
cargo install miden-client
```

Check the official documentation for more details reagarding:

- [Library Features](https://docs.polygon.technology/miden/miden-client/library/#features)
- [Configuration](https://docs.polygon.technology/miden/miden-client/cli-config/)
- [Examples](https://0xpolygonmiden.github.io/miden-base/introduction/getting-started.html)
- [CLI Reference](https://docs.polygon.technology/miden/miden-client/cli-reference/#types-of-transaction)

### Makefile

As mentioned before, we use `cargo-make` to encapsulate some tasks, such as running lints and tests. You can check out [Makefile.toml](./Makefile.toml) for all possible tasks to run.

## Testing

To test the project's code, we provide both unit tests (which can be run with `cargo test`) and integration tests. For more info on integration tests, refer to the [integration testing document](./tests/README.md)

## Changelog

You can keep track of the client's changes [here](./CHANGELOG.md).

## Contributing

Interested in contributing with the project? Check [CONTRIBUTING.md](./CONTRIBUTING.md).

## License
This project is [MIT licensed](./LICENSE).
