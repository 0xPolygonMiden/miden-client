# Overview

This document describes the current state of the organization of integration tests, along with info on how to run them.

## Running integration tests

There are commands provided in the `Makefile` to make running them easier. To run the current integration test, you should run:

```bash
# This will ensure we start from a clean node and client
make reset
# This command will clone the node's repo and generate the accounts and genesis files and lastly start the node and run it on background
make start-node &
# This will run the integration test and after it finishes it will kill the node process
make integration-test
```

## Integration Test Flow

The integration test goes through a series of supported flows such as minting
and transfering assets which runs against a running node. 

### Setup

Before running the tests though, there is a setup we need to perform to have a
node up and running. This is accomplished with the `node` command from the
`Makefile` and what it does is:

- clone the node repo if it doesn't exist
- delete previously existing data
- generate genesis and account data with `cargo run --release --bin miden-node --features testing -- make-genesis --inputs-path node/genesis.toml`

After that we can start the node, again done in the `start-node` command from the `Makefile`

### Test Run

The integration test is run with the `integration-test` command from the
Makefile, although it's actually a proxy for the `run_integration_test.sh`
script. What that script takes care of is:

- Compiling the integration test code
- Waiting for the node to be up (this allows us to run both the `start-node`
  and `integration-test` commands without worrying about synchronization
  issues)
- Run the integration test binary
- Kill the node process

### The test itself

The current integration test at `./integration/main.rs` goes through the following steps:

1. Load accounts (1 regular *A*, 1 faucet *C*) created with the `make-genesis` command of the node
2. Create an extra regular account *C*
3. Sync up the client
4. Mint an asset (this creates a note for the regular account *A*) and sync again
5. Consume the note and sync again. (After this point the account *A* should have an asset from faucet *C*)
6. Run a P2ID transaction to transfer some of the minted asset from account *A* to *B*. Sync again
7. Consume the P2ID note for account *B*. Now both accounts should have some of asset from faucet *C*

In short, we're testing:

- account importing
- account creation
- sync
- mint tx
- consume note tx (both for an imported and a created account)
- P2ID tx

## CI integration

There is a step for the CI at `../.github/workflows/ci.yml` used to run the integration tests.
