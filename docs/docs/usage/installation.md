# Usage

Before using Miden Client, make sure you have Rust [installed](https://www.rust-lang.org/tools/install). Miden Client requires Rust version **1.67** or later.

As mentioned in the Overview section, the Client is comprised by a library and a CLI that exposes its main functionality.

## CLI interface

### Installing the CLI

The first step to installing the Client, is to clone the [Client's repository](https://github.com/0xPolygonMiden/miden-client/).
You can then choose to run the client CLI using `cargo run`, or install it on your system. The current recommended way of installing and running the client is to utilize the `testing` and `concurrent` features:

```sh
cargo install --features testing,concurrent --path .
```

This will install the `miden-client` binary (at `~/.cargo/bin/miden-client`) and add it to your `PATH`. 

### Features

#### `Testing` feature

The `testing` feature allows mainly for faster account creation. When using the the client CLI alongside a locally-running node, **you will need to make sure the node is installed/executed with the `testing` feature as well**, as some validations can fail if flag does not match up both on the client and the node.

#### `Concurrent` feature

Additionally, the client supports another feature: The `concurrent` flag enables optimizations that will result in faster transaction execution and proving.
