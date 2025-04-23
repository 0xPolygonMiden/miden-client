# Synchronizing State with the Miden SDK

This guide demonstrates how to synchronize your local state with the Miden network using the SDK. Synchronization ensures that your local data (accounts, notes, transactions) is up-to-date with the network.

## Basic Synchronization

To synchronize your local state with the network:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    const syncSummary = await webClient.syncState();
    
    // Access synchronization details
    console.log("Current block number:", syncSummary.blockNum());
    console.log("Committed notes:", syncSummary.committedNotes().map(id => id.toString()));
    console.log("Consumed notes:", syncSummary.consumedNotes().map(id => id.toString()));
    console.log("Updated accounts:", syncSummary.updatedAccounts().map(id => id.toString()));
    console.log("Committed transactions:", syncSummary.committedTransactions().map(id => id.toString()));
} catch (error) {
    console.error("Failed to sync state:", error.message);
}
```

## Understanding the Sync Summary

The `SyncSummary` object returned by `syncState()` contains the following information:

- `blockNum()`: The current block number of the network
- `committedNotes()`: Array of note IDs that have been committed to the network
- `consumedNotes()`: Array of note IDs that have been consumed
- `updatedAccounts()`: Array of account IDs that have been updated
- `committedTransactions()`: Array of transaction IDs that have been committed

## Relevant Documentation

For more detailed information about sync functionality, refer to the following API documentation:

- [WebClient](docs/src/web-client/api/classes/WebClient.md) - Main client class for sync operations
- [SyncSummary](docs/src/web-client/api/classes/SyncSummary.md) - Class representing sync state
- [NoteId](docs/src/web-client/api/classes/NoteId.md) - Class for working with note IDs
- [AccountId](docs/src/web-client/api/classes/AccountId.md) - Class for working with account IDs
- [TransactionId](docs/src/web-client/api/classes/TransactionId.md) - Class for working with transaction IDs

For a complete list of available classes and utilities, see the [SDK API Reference](docs/src/web-client/api/README.md). 