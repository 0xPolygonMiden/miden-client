## Software prerequisites

- [Rust installation](https://www.rust-lang.org/tools/install) minimum version 1.82.

## Install the client

We currently recommend installing and running the client with the [`testing`](#testing-feature) and [`concurrent`](#concurrent-feature) features.

Run the following command to install the miden-client:

```sh
cargo install miden-cli --features concurrent,testing
```

This installs the `miden` binary (at `~/.cargo/bin/miden`) with the [`testing`](#testing-feature) and [`concurrent`](#concurrent-feature) features.

### `Testing` feature

The `testing` feature speeds up account creation. 

> **Warning** "Install the `testing` feature on node and client"
> - When using the client CLI alongside a locally-running node, make sure to install/execute the node with the `testing` feature. 
> - Some validations can fail if the flag does not match on both the client and the node.

### `Concurrent` feature

The `concurrent` flag enables optimizations that result in faster transaction execution and proving times.

## Run the client 

1. Make sure you have already [installed the client](#install-the-client). If you don't have a `miden-client.toml` file in your directory, create one or run `miden init` to initialize one at the current working directory. You can do so without any arguments to use its defaults or define either the RPC config or the store config via `--rpc` and `--store-path`

2. Run the client CLI using:

    ```sh
    miden
    ```
