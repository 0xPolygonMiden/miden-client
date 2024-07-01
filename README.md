# Miden client

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/0xPolygonMiden/miden-client/blob/main/LICENSE)
[![test](https://github.com/0xPolygonMiden/miden-client/actions/workflows/test.yml/badge.svg)](https://github.com/0xPolygonMiden/miden-clinet/actions/workflows/test.yml)
[![RUST_VERSION](https://img.shields.io/badge/rustc-1.78+-lightgray.svg)](https://www.rust-lang.org/tools/install)
[![crates.io](https://img.shields.io/crates/v/miden-client)](https://crates.io/crates/miden-client)

This repository contains the Miden client, which provides a way to execute and prove transactions, facilitating the interaction with the Miden rollup.

### Status

The Miden client is still under heavy development and the project can be considered to be in an *alpha* stage. Many features are yet to be implemented and there is a number of limitations which we will lift in the near future.

## Overview

The Miden client currently consists of two components:

- `miden-client` library, which can be used by other project to programmatically interact with the Miden rollup. 
- `miden-client` binary which is a wrapper around the library exposing its functionality via a simple command-line interface (CLI).

The client's main responsibility is to maintain a partial view of the blockchain which allows for locally executing and proving transactions. It keeps a local store of various entities that periodically get updated by syncing with the node.

For more info check:

- [Getting started](https://0xpolygonmiden.github.io/miden-base/introduction/getting-started.html)
- [CLI Reference](./docs/cli-reference.md#types-of-transaction)
    - [Configuration](./docs/cli-config.md)
- [Online Documentation](https://docs.polygon.technology/miden/miden-client)

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
