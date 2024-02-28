# Overview
## Components:

The Miden client currently consists of two main components:

1. **Miden client Library:** A Rust library that can be integrated to projects, allowing developers to programmatically interact with the Miden rollup. It provides a set of APIs and functions for executing transactions, generating proofs, and managing interactions with the Miden network.
2. **Miden client CLI:** The Miden client also includes a command-line interface (CLI) that serves as a wrapper around the library, exposing its basic functionality in a user-friendly manner. It allows users to execute various commands to interact with the Miden rollup, such as submitting transactions, syncing with the network, and managing account data.

## Key features:

The Miden client offers a range of functionality to correctly interact with the Miden rollup:

- **Transaction Execution:** The Miden client facilitates the execution of transactions on the Miden rollup, allowing users to transfer assets, mint new tokens, and perform various other operations.
- **Proof Generation:** The Miden rollup allows for user-generated proofs, so the client contains functionality to execute, prove and submit transactions. These proofs are key to ensuring the validity of transactions on the Miden rollup.
- **Interaction with the Miden Network:** The Miden client enables users to interact with the Miden network, syncing with the latest blockchain data and managing account information.
- **Account Generation and tracking**: The Miden client provides features for generating and tracking accounts within the Miden rollup ecosystem. Users can create accounts and track their changes based on transactions.
