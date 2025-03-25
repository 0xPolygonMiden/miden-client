## Software prerequisites

- [Rust installation](https://www.rust-lang.org/tools/install) minimum version 1.85.

## Install the client

We currently recommend installing and running the client with the [`concurrent`](#concurrent-feature) feature.

Run the following command to install the miden-client:

```sh
cargo install miden-cli --features concurrent
```

This installs the `miden` binary (at `~/.cargo/bin/miden`) with the [`concurrent`](#concurrent-feature) feature.

### `Concurrent` feature

The `concurrent` flag enables optimizations that result in faster transaction execution and proving times.

## Run the client 

1. Make sure you have already [installed the client](#install-the-client). If you don't have a `miden-client.toml` file in your directory, create one or run `miden init` to initialize one at the current working directory. You can do so without any arguments to use its defaults or define either the RPC endpoint or the store config via `--network` and `--store-path`

2. Run the client CLI using:

    ```sh
    miden
    ```
