The Miden client has the following architectural components:

- [Store](#store)
- [RPC client](#rpc-client)
- [Transaction executor](#transaction-executor)

> [!Tip]
> - The RPC client and the store are Rust traits.
> - This allow developers and users to easily customize their implementations.

## Store

The store is central to the client's design. 

It manages the persistence of the following entities:

- Accounts; including their state history and related information such as vault assets and account code.
- Transactions and their scripts.
- Notes.
- Note tags.
- Block headers and chain information that the client needs to execute transactions and consume notes.
 
Because Miden allows off-chain executing and proving, the client needs to know about the state of the blockchain at the moment of execution. To avoid state bloat, however, the client does not need to see the whole blockchain history, just the chain history intervals that are relevant to the user. 

The store can track any number of accounts, and any number of notes that those accounts might have created or may want to consume. 

## RPC client

The RPC client communicates with the node through a defined set of gRPC methods. 

Currently, these include:

- `GetBlockHeaderByNumber`: Returns the block header information given a specific block number.
- `SyncState`: Asks the node for information relevant to the client. For example, specific account changes, whether relevant notes have been created or consumed, etc.
- `SubmitProvenTransaction`: Sends a locally-proved transaction to the node for inclusion in the blockchain.

## Transaction executor

The transaction executor executes transactions using the Miden VM. 

When executing, the executor needs access to relevant blockchain history. The executor uses a `DataStore` interface for accessing this data. This means that there may be some coupling between the executor and the store.
