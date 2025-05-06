# Miden client

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/0xMiden/miden-client/blob/main/LICENSE)
[![test](https://github.com/0xMiden/miden-client/actions/workflows/test.yml/badge.svg)](https://github.com/0xMiden/miden-client/actions/workflows/test.yml)
[![build](https://github.com/0xMiden/miden-client/actions/workflows/build.yml/badge.svg)](https://github.com/0xMiden/miden-client/actions/workflows/build.yml)
[![RUST_VERSION](https://img.shields.io/badge/rustc-1.86+-lightgray.svg)](https://www.rust-lang.org/tools/install)
[![crates.io](https://img.shields.io/crates/v/miden-client)](https://crates.io/crates/miden-client)

This repository contains the Miden client, which provides a way to execute and prove transactions, facilitating the interaction with the Miden rollup.

### Status

The Miden client is still under heavy development and the project can be considered to be in an *alpha* stage. Many features are yet to be implemented and there is a number of limitations which we will lift in the near future.

## Overview

The Miden client currently consists of two components:

- `miden-client` library, which can be used by other project to programmatically interact with the Miden rollup. You can find more information about the library in the [Rust client Library](./crates/rust-client/README.md) section.
- `miden-cli`, which is a wrapper around the library exposing its functionality via a simple command-line interface (CLI). You can find more information about the CLI in the [Miden client CLI](./bin/miden-cli/README.md) section.

The client's main responsibility is to maintain a partial view of the blockchain which allows for locally executing and proving transactions. It keeps a local store of various entities that periodically get updated by syncing with the node.

For more info check:

- [Getting started](https://0xMiden.github.io/miden-docs/miden-client/get-started/prerequisites.html)
- [CLI Reference](https://0xMiden.github.io/miden-docs/miden-client/cli-reference.html)
- [Configuration](https://0xMiden.github.io/miden-docs/miden-client/cli-config.html)
- [Online Documentation](https://0xMiden.github.io/miden-docs/miden-client/index.html)

## Workspace structure

The workspace is organized as follows:
- The `bin` folder contains crates that are meant to be compiled into binaries (like the CLI).
- The `crates` folder contains the library crates that are meant to be used as dependencies (like the Rust client library).
- The `tests` folder contains integration tests for the workspace crates.

### Makefile

We use `make` to encapsulate some tasks, such as running lints and tests. You can check out [Makefile](./Makefile) for all available tasks or just run the following command:

```bash
make
```

## Testing

To test the project's code, we provide both unit tests (which can be run with `cargo test`) and integration tests. For more info on integration tests, refer to the [integration testing document](./tests/README.md)

The crate also comes with one feature flag that is used exclusively on tests: 

- `integration`: only used to run integration tests and separate them from unit tests.

## Contributing

Interested in contributing? Check [CONTRIBUTING.md](./CONTRIBUTING.md).

## License
This project is [MIT licensed](./LICENSE).
