# Miden client

<a href="https://github.com/0xPolygonMiden/miden-node/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
<a href="https://github.com/0xPolygonMiden/miden-client/actions/workflows/ci.yml"><img src="https://github.com/0xPolygonMiden/miden-client/actions/workflows/ci.yml/badge.svg?branch=main"></a>
<a href="https://crates.io/crates/miden-client"><img src="https://img.shields.io/crates/v/miden-client"></a>

This repository contains the Miden client, which provides a way to execute and prove transactions, facilitating the interaction with the Miden rollup.

### Status

The Miden client is still under heavy development and the project can be considered to be in an *alpha* stage. Many features are yet to be implemented and there is a number of limitations which we will lift in the near future.

## Overview

The Miden client currently consists of two components:

- `miden-client` library, which can be used by other project to programmatically interact with the Miden rollup. 
- `miden-client` binary which is a wrapper around the library exposing its functionality via a simple command-line interface (CLI).

The client's main responsibility is to maintain a partial view of the blockchain which allows for locally executing and proving transactions. It keeps a local store of various entities that periodically get updated by syncing with the node.

## Usage

### Installing the CLI

Before you can build and run the Miden client CLI, you'll need to make sure you have Rust [installed](https://www.rust-lang.org/tools/install). Miden client v0.1 requires Rust version **1.75** or later.

You can then install the CLI on your system:

```sh
cargo install miden-client 
```

This will install the `miden-client` binary on `$HOME/.cargo/bin` by default. 

For testing, the following way of installing is recommended:

```sh
cargo install miden-client --features testing,concurrent
```

The `testing` feature allows mainly for faster account creation. When using the the client CLI alongside a locally-running node, you will want to make sure the node is installed/executed with the `testing` feature as well, as some validations can fail if the settings do not match up both on the client and the node.

Additionally, the `concurrent` flag enables optimizations that will result in faster transaction execution and proving.

After installing the client, you can use it by running `miden-client`. In order to get more information about available CLI commands you can run `miden-client --help`.

> [!IMPORTANT]  
> In order to make transaction execution and proving faster, it's important that the client is built on release mode. When using `cargo install`, this is the default build configuration. However, in case you want to run the client using `cargo run`, this needs to be explicitly set (`cargo run --release --features testing, concurrent`).

### Connecting to the network

The CLI can be configured through a TOML file ([`miden-client.toml`](miden-client.toml)). This file is expected to be located in the directory from where you are running the CLI. This is useful for connecting to a specific node when developing with the client, for example. 

In the configuration file, you will find a section for defining the node's endpoint and the store's filename. By default, the node will run on `localhost:57291`, so the example file defines this as the RPC endpoint.

## Example: Executing, proving and submitting transactions

### Prerequisites

- This guide assumes a basic understanding of the Miden rollup, as it deals with some of its main concepts, such as Notes, Accounts, and Transactions. A good place to learn about these concepts is the [Polygon Miden Documentation](https://0xpolygonmiden.github.io/miden-base/introduction.html).
- It also assumes that you have set up a [Miden Node](https://github.com/0xPolygonMiden/miden-node) that can perform a basic local transaction. 
- Currently, the client allows for submitting locally-proven transactions to the Miden node. The simplest way to test the client is by [generating accounts via the genesis file](https://github.com/0xPolygonMiden/miden-node?tab=readme-ov-file#generating-the-genesis-file). 
  - For this example, we will make use of 1 faucet account and 2 regular wallet accounts, so you should set your node's `toml` config file accordingly. We will refer to these accounts as having IDs `regular account ID A` and `regular account ID B` in order differentiate them.
  - Once the account files have been generated, [make sure the node is running](https://github.com/0xPolygonMiden/miden-node?tab=readme-ov-file#running-the-node). If the node has some stored state from previous tests and usage, you might have to clear its database (`miden-store.sqlite3`).
  - The client should be configured to use the running node's socket as its endpoint as explained in the previous section.

### 1. Loading account data

In order to execute transactions and change the account's states, you will first want to import the generated account information by running `miden-client account import`:

```bash
miden-client account import <path-to-accounts-directory>/*.mac
```

The client will then import all account-related data generated by the node (stored as `*.mac` files), and insert them in the local store. The accounts directory should be the one generated by the node when running the `make-genesis` command. You can now list the imported accounts by running:

```bash
miden-client account list
```

### 2. Synchronizing the state

As briefly mentioned in the [Overview](#overview) section, the client needs to periodically query the node to receive updates about entities that might be important in order to run transactions. The way to do so is by running the `sync` command:

```bash
miden-client sync
```

Running this command will update local data up to the chain tip. This is needed in order to execute and prove any transaction.

### 3. Minting an asset 

Since we have now synced our local view of the blockchain and have account information, we are ready to execute and submit tranasctions. For a first test, we are going to mint a fungible asset for a regular account.

```bash
miden-client tx new mint <regular-account-ID-A> <faucet-account-id> 1000
```

This will execute, prove and submit a transaction that mints assets to the node. The account that executes this transaction will be the faucet as was defined in the node's configuration file. In this case, it is minting `1000` fungible tokens to `<regular-account-ID-A>`. 

This will add a transaction and an output note (containing the minted asset) to the local store in order to track their lifecycles. You can display them by running `miden-client tx list` and `miden-client input-notes list` respectively. If you do so, you will notice that they do not show a `commit height` even though they were submitted to the operator. This is because our local view of the network has not yet been updated, so the client does not have a way to prove the inclusion of the note in the blockchain. After updating it with a `sync`, you should see the height at which the transaction and the note containing the asset were committed. This will allow us to prove transactions that make use of this note, as we can compute valid proofs that state that the note exists in the blockchain.

### 4. Consuming the note

After creating the note with the minted asset, the regular account can now consume it and add the tokens to its vault. You can do this the following way:

```bash
miden-client tx new consume-notes <regular-account-ID-A> <input-note-ID>
```

This will consume the input note identified by its ID, which you can get by listing them as explained in the previous step. Note that you can consume more than one note in a single transaction. Additionally, it's possible to provide just a prefix of a note's ID. For example, instead of `miden-client tx new consume-notes <regular-account-ID-A> 0x70b7ecba1db44c3aa75e87a3394de95463cc094d7794b706e02a9228342faeb0` you can do `miden-client tx new consume-notes <regular-account-ID-A> 0x70b7ec`. 

You will now be able to see the asset in the account's vault by running:

```bash
miden-client account show <regular-account-ID-A> -v
```

### 5. Transferring assets between accounts

Some of the tokens we minted can now be transferred to our second regular account. To do so, you can run:

```bash
miden-client sync # Make sure we have an updated view of the state
miden-client tx new p2id <regular-account-ID-A> <regular-account-ID-B> <faucet-account-ID> 50 # Transfers 50 tokens to account ID B
```

This will generate a Pay-to-ID (`P2ID`) note containing 50 assets, transferred from one regular account to the other. You can see the new note by running `miden-client input-notes list`. If we sync, we can now make use of the note and consume it for the receiving account:

```bash
miden-client sync # Make sure we have an updated view of the state
miden-client tx new consume-notes <regular-account-ID-B> <input-note-ID> # Consume the note
```

That's it! You will now be able to see `950` fungible tokens in the first regular account, and `50` tokens in the remaining regular account:

```bash
miden-client account show <regular-account-ID-B> -v # Show account B's vault assets (50 fungible tokens)
miden-client account show <regular-account-ID-A> -v # Show account A's vault assets (950 fungible tokens)
```

### Clearing the state

All state is maintained in `store.sqlite3`, located in the same directory where the client binary is. In case it needs to be cleared, the file can be deleted; it will later be created again when any command is executed.


## Utilizing the library

In order to utilize the `miden-client` library, you can add the dependency to your project's `Cargo.toml` file:

````toml
miden-client = { version = "0.1", features = ["concurrent", "testing"]  }
````

## Testing

This crate has both unit tests (which can be run with `cargo test`) and integration tests. For more info on integration tests, refer to the [integration testing document](./tests/README.md)

## License
This project is [MIT licensed](./LICENSE).
