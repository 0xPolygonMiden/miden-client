# Usage

Before using Miden Client, make sure you have Rust [installed](https://www.rust-lang.org/tools/install). Miden Client requires Rust version **1.67** or later.

As mentioned in the Overview section, the Client is comprised by a library and a CLI that exposes its main functionality.

## CLI interface

### Installing the CLI

The first step to installing the Client, is to clone the [Client's repository](https://github.com/0xPolygonMiden/miden-client/).
You can then choose to run the client CLI using `cargo`, or install it on your system. In order to install it, you can run:

```sh
cargo install --path .
```

This will install the `miden-client` binary in your PATH, at `~/.cargo/bin/miden-client`. 

### Optional features

#### `Testing` feature
For testing, the following way of installing is recommended:

```sh
cargo install --features testing --path .
```

The `testing` feature allows mainly for faster account creation. When using the the client CLI alongside a locally-running node, you will want to make sure the node is installed/executed with the `testing` feature as well, as some validations can fail if the settings do not match up both on the client and the node.

#### `Concurrent` feature

Additionally, the client supports another feature: The `concurrent` flag enables optimizations that will result in faster transaction execution and proving.

```sh
cargo install --features concurrent --path .
```