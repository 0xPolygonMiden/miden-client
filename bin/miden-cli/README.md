# Miden Client CLI

This binary is a wrapper around the library exposing its functionality via a simple command-line interface (CLI).

## Usage

Before you can use the Miden client, you'll need to make sure you have both
[Rust](https://www.rust-lang.org/tools/install) and sqlite3 installed. Miden
client requires rust version **1.78** or higher.

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

These actions can also be executed via the Makefile with `make build` or `make install`.
