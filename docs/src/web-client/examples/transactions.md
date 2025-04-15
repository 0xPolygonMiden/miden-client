# Retrieving Transaction History with the Miden SDK

This guide demonstrates how to retrieve and work with transaction history using the Miden SDK. We'll cover different ways to query transactions and access their properties. Each example includes detailed annotations to explain the key parameters and returned properties.

## Basic Transaction Retrieval

To get a list of all transactions:

```typescript
import { TransactionFilter, WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Get all transactions
    const allTransactions = await webClient.getTransactions(TransactionFilter.all());

    // Iterate through transactions
    for (const tx of allTransactions) {
        console.log(tx.id().toString());           // Transaction ID
        console.log(tx.accountId().toString());    // Account ID associated with the transaction
        console.log(tx.blockNum().toString());     // Block number where the transaction was included
        
        // Check transaction status
        const status = tx.transactionStatus();
        if (status.isPending()) {
            console.log("Status: Pending");
        } else if (status.isCommitted()) {
            console.log("Status: Committed in block", status.getBlockNum());
        } else if (status.isDiscarded()) {
            console.log("Status: Discarded");
        }
        
        // Account state changes
        console.log("Initial Account State:", tx.initAccountState().toHex());
        console.log("Final Account State:", tx.finalAccountState().toHex());
        
        // Input and output notes
        console.log("Input Note Nullifiers:", tx.inputNoteNullifiers().map(n => n.toHex()));
        console.log("Output Notes:", tx.outputNotes().toString());
    }
} catch (error) {
    console.error("Failed to retrieve transactions:", error.message);
}
```

## Retrieving Uncommitted Transactions

To get transactions that haven't been committed to the blockchain yet:

```typescript
import { TransactionFilter, WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Get uncommitted transactions
    const uncommittedTransactions = await webClient.getTransactions(TransactionFilter.uncomitted());

    // Process transactions as needed
    for (const tx of uncommittedTransactions) {
        console.log("Uncommitted Transaction:", tx.id().toString());
        const status = tx.transactionStatus();
        if (status.isPending()) {
            console.log("Status: Pending");
        } else if (status.isDiscarded()) {
            console.log("Status: Discarded");
        }
    }
} catch (error) {
    console.error("Failed to retrieve uncommitted transactions:", error.message);
}
```

## Working with Transaction Records

Each transaction record contains detailed information about the transaction:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";\

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    const transactions = await webClient.getTransactions(TransactionFilter.all());

    for (const tx of transactions) {
        // Basic transaction info
        console.log("Transaction ID:", tx.id().toString());
        console.log("Account ID:", tx.accountId().toString());
        console.log("Block Number:", tx.blockNum().toString());
        
        // Transaction status
        const status = tx.transactionStatus();
        if (status.isPending()) {
            console.log("Status: Pending");
        } else if (status.isCommitted()) {
            console.log("Status: Committed in block", status.getBlockNum());
        } else if (status.isDiscarded()) {
            console.log("Status: Discarded");
        }
        
        // Account state changes
        console.log("Initial State:", tx.initAccountState().toHex());
        console.log("Final State:", tx.finalAccountState().toHex());
        
        // Notes information
        console.log("Input Note Nullifiers:", tx.inputNoteNullifiers().map(n => n.toHex()));
        console.log("Output Notes:", tx.outputNotes().toString());
    }
} catch (error) {
    console.error("Failed to process transaction records:", error.message);
}
```

## Transaction Statuses

Transactions can have the following statuses:
- `pending`: Transaction is waiting to be processed
- `committed`: Transaction has been successfully processed and included in a block (includes block number)
- `discarded`: Transaction was discarded and will not be processed

You can check the status of a transaction using the following methods:
- `isPending()`: Returns true if the transaction is pending
- `isCommitted()`: Returns true if the transaction is committed
- `isDiscarded()`: Returns true if the transaction is discarded
- `getBlockNum()`: Returns the block number if the transaction is committed, otherwise returns null

## Relevant Documentation

For more detailed information about the classes and methods used in these examples, refer to the following API documentation:

- [WebClient](docs/src/web-client/api/classes/WebClient.md) - Main client class for interacting with the Miden network
- [TransactionRecord](docs/src/web-client/api/classes/TransactionRecord.md) - Historical transaction records and their properties
- [TransactionFilter](docs/src/web-client/api/classes/TransactionFilter.md) - Transaction filtering options
- [TransactionId](docs/src/web-client/api/classes/TransactionId.md) - Transaction identifier class
- [AccountId](docs/src/web-client/api/classes/AccountId.md) - Account identifier class
- [RpoDigest](docs/src/web-client/api/classes/RpoDigest.md) - Commitment hash class used for various transaction properties
- [OutputNotes](docs/src/web-client/api/classes/OutputNotes.md) - Output notes associated with transactions
- [TransactionStatus](docs/src/web-client/api/classes/TransactionStatus.md) - Transaction status information and methods

For a complete list of available classes and utilities, see the [SDK API Reference](docs/src/web-client/api/README.md). 