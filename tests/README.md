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

- Clone the node repo if it doesn't exist.
- Delete previously existing data.
- Generate genesis and account data with `cargo run --release --bin miden-node --features testing -- make-genesis --inputs-path node/genesis.toml`.

After that we can start the node, again done in the `start-node` command from the `Makefile`

### Test Run

To run the integration test you just need to run `make integration-test`. It'll
run the rust binary for the integration test and report whether there was an
error or not and the exit code if so. Lastly it kills the node's process.

### The test itself

The current integration test at `./integration/main.rs` goes through the following steps:

0. Wait for the node to be reachable (this is mainly so you can run `make start-node` 
   and `make integration-test` in parallel without major issues).
   This is done with a sync request, although in the future we might use a
   health check endpoint or similar.
1. Load accounts (1 regular *A*, 1 faucet *C*) created with the `make-genesis`
   command of the node
2. Create an extra regular account *C*
3. Sync up the client
4. Mint an asset (this creates a note for the regular account *A*) and sync
   again
5. Consume the note and sync again. (After this point the account *A* should
   have an asset from faucet *C*)
6. Run a P2ID transaction to transfer some of the minted asset from account *A*
   to *B*. Sync again
7. Consume the P2ID note for account *B*. Now both accounts should have some of
   asset from faucet *C*

In short, we're testing:

- Account importing.
- Account creation.
- Sync.
- Mint tx.
- Consume note tx (both for an imported and a created account).
- P2ID tx.

## CI integration

There is a step for the CI at `../.github/workflows/ci.yml` used to run the integration tests.
