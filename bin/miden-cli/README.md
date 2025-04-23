# Miden client CLI

This binary allows the user to interact with the Miden rollup via a simple command-line interface (CLI). It's a wrapper around the [Miden client](https://crates.io/crates/miden-client) library exposing its functionality in order to create accounts, create and consume notes, all executed and proved using the Miden VM.

## Usage

Before you can use the Miden client, you'll need to make sure you have both [Rust](https://www.rust-lang.org/tools/install) and SQLite3 installed. Miden client requires rust version **1.86** or higher.

### Running `miden-client`'s CLI

You can either build from source with:

```bash
cargo build --release --features "concurrent"
```

The `concurrent` feature is enabled to improve execution and proving times.

Once the binary is built, you can find it on `./target/release/miden`.

Or you can install the CLI from crates.io with:

```bash
cargo install --features "concurrent" miden-cli
```

These actions can also be executed when inside the repository via the Makefile with `make build` or `make install`.

### Using the CLI

To have a fully-functional client CLI, you would need to set it up first. You can accomplish that with:

```shell
miden init
```

This would generate the `miden-client.toml` file, which contains useful information for the client like RPC provider's URL and database path.

After this, your client should be set and ready to use. Get the available commands with:

```shell
miden
# or
miden --help
```

The first time that you sync your client (`miden sync`) a new file will be generated based on the configurations set on `miden-client.toml`. This file is the database of the client.

## License
This project is [MIT licensed](../../LICENSE).
