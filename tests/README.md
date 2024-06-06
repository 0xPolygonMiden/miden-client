# Overview

This document describes the current state of the organization of integration tests, along with info on how to run them.

## Running integration tests

There are commands provided in the `Makefile.toml` to make running them easier. To run the current integration test, you should run:

```bash
# This will ensure we start from a clean node and client
cargo make reset
# This command will clone the node's repo and generate the accounts and genesis files and lastly start the node 
cargo make node
# This command will run the node on background
cargo make start-node &
# This will run the integration test 
cargo make integration-test-full
```

## Integration Test Flow

The integration test goes through a series of supported flows such as minting
and transferring assets which runs against a running node. 

### Setup

Before running the tests though, there is a setup we need to perform to have a
node up and running. This is accomplished with the `node` command from the
`Makefile.toml` and what it does is:

- Clone the node repo if it doesn't exist.
- Delete previously existing data.
- Generate genesis and account data with `cargo run --release --bin miden-node --features testing -- make-genesis --inputs-path node/genesis.toml`.

After that we can start the node, again done in the `start-node` command from
the `Makefile.toml`. Killing the node process after running the test is also
the user's responsibilty.

### Test Run

To run the integration test you just need to run `cargo make integration-test`.
It'll run the integration tests as a cargo test using the `integration` feature
which is used to separate regular tests from integration tests.

### Ignored Tests

Currently, we have one ignored test because it requires having the genesis data
from the node it is running against which might not always be possible. You can
run it manually by doing:

```bash
cargo nextest run --profile ci-default --release --test=integration --features integration --run-ignored ignored-only -- test_import_genesis_accounts_can_be_used_for_transactions
```

Or run `cargo make integration-test-full` to run all integration tests with
that included. On the other hand, if you want to run integration tests without
that one you can just instead do:

```bash
cargo make integration-test
```

### Running tests against a remote node

You can run the integration tests against a remote node by overwriting the rpc
section of the configuration file at `./config/miden-client.toml`. 
## CI integration

There is a step for the CI at `../.github/workflows/ci.yml` used to run the integration tests.
