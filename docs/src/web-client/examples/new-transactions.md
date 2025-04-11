# Creating Transactions with the Miden SDK

This guide demonstrates how to create and submit different types of transactions using the Miden SDK. We'll cover minting, sending, consuming, and custom transactions.

## Basic Transaction Flow

All transactions follow a similar pattern:
1. Create a transaction request
2. Execute the transaction
3. Submit the transaction to the network

Here's a basic example of how to execute and submit a mint transaction to mint tokens from a faucet:

```typescript
import { NoteType, WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    const transactionRequest = webClient.newMintTransactionRequest(
        targetAccountId, // AccountId: The account that will receive the minted tokens
        faucetId,// AccountId: The faucet account that will mint the tokens
        NoteType.Private, // NoteType: The type of note to create (Private or Public)
        1000 // number: The amount of tokens to mint
    );

    // 2. Execute transaction
    const transactionResult = await webClient.newTransaction(
        accountId,
        transactionRequest
    );

    // 3. Submit transaction
    await webClient.submitTransaction(transactionResult);
    
    // Access transaction details
    console.log("Block number:", transactionResult.blockNum());
    console.log("Created notes:", transactionResult.createdNotes());
    console.log("Consumed notes:", transactionResult.consumedNotes());
    console.log("Account delta:", transactionResult.accountDelta());
} catch (error) {
    console.error("Transaction failed:", error.message);
}
```

### Using a Remote Prover

For better performance, you can offload the work of proving the transaction to a remote prover. This is especially useful for complex transactions:

```typescript
import { NoteType, TransactionProver, WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Create a remote prover with the endpoint
    const remoteProver = TransactionProver.newRemoteProver("https://prover.example.com");

    // 1. Create transaction request
    const transactionRequest = webClient.newMintTransactionRequest(
        targetAccountId, // AccountId: The account that will receive the minted tokens
        faucetId,// AccountId: The faucet account that will mint the tokens
        NoteType.Private, // NoteType: The type of note to create (Private or Public)
        1000 // number: The amount of tokens to mint
    );

    // 2. Execute transaction
    const transactionResult = await webClient.newTransaction(
        accountId,
        transactionRequest
    );

    // 3. Submit transaction with remote prover
    await webClient.submitTransaction(transactionResult, remoteProver);
    
    // Access transaction details
    console.log("Block number:", transactionResult.blockNum());
    console.log("Created notes:", transactionResult.createdNotes());
    console.log("Consumed notes:", transactionResult.consumedNotes());
    console.log("Account delta:", transactionResult.accountDelta());
} catch (error) {
    console.error("Transaction failed:", error.message);
}
```

> **Note**: Using a remote prover can significantly improve performance for complex transactions by offloading the computationally intensive proving work to a dedicated server. This is particularly useful when dealing with large transactions or when running in resource-constrained environments.

## Sending Transactions

To send tokens between accounts:

```typescript
import { NoteType, WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    const transactionRequest = webClient.newSendTransactionRequest(
        senderAccountId,  // Account sending tokens
        targetAccountId,  // Account receiving tokens
        faucetId,        // Faucet account ID
        NoteType.Private, // Note type
        100,             // Amount to send
        100             // Optional recall height
    );

    const transactionResult = await webClient.newTransaction(
        senderAccountId,
        transactionRequest
    );

    await webClient.submitTransaction(transactionResult);
    
    // Access transaction details
    console.log("Block number:", transactionResult.blockNum());
    console.log("Created notes:", transactionResult.createdNotes());
    console.log("Consumed notes:", transactionResult.consumedNotes());
    console.log("Account delta:", transactionResult.accountDelta());
} catch (error) {
    console.error("Send transaction failed:", error.message);
}
```

## Consuming Notes

To consume (spend) notes:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    const transactionRequest = webClient.newConsumeTransactionRequest(
        [noteId1, noteId2]  // Array of note IDs to consume
    );

    const transactionResult = await webClient.newTransaction(
        accountId,
        transactionRequest
    );

    await webClient.submitTransaction(transactionResult);
    
    // Access transaction details
    console.log("Block number:", transactionResult.blockNum());
    console.log("Created notes:", transactionResult.createdNotes());
    console.log("Consumed notes:", transactionResult.consumedNotes());
    console.log("Account delta:", transactionResult.accountDelta());
} catch (error) {
    console.error("Consume transaction failed:", error.message);
}
```

## Custom Transactions

For advanced use cases, you can create custom transactions by defining your own note scripts and transaction parameters. This allows for:

- Custom note validation logic
- Complex asset transfers
- Custom authentication schemes
- Integration with smart contracts

> **Note**: For a complete example of a custom transaction implementation, including input notes, output notes, and custom scripts, see the integration tests in [`new_transactions.test.ts`](https://github.com/0xPolygonMiden/miden-client/blob/main/crates/web-client/test/new_transactions.test.ts).

Here's a simplified example of creating a custom transaction:

```typescript
import { 
    Felt, 
    FeltArray,
    FungibleAsset,
    NotesArray
    NoteAssets,
    NoteExecutionHint,
    NoteExecutionMode,
    NoteMetadata, 
    NoteTag,
    NoteType, 
    OutputNotesArray,
    TransactionRequestBuilder,
    TransactionScript,
    WebClient
} from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Create note assets
    const noteAssets = new NoteAssets([
        new FungibleAsset(faucetId, BigInt(10))
    ]);

    // Create note metadata
    const noteMetadata = new NoteMetadata(
        faucetId,
        NoteType.Private,
        NoteTag.fromAccountId(targetAccountId, NoteExecutionMode.newLocal()),
        NoteExecutionHint.none()
    );

    // Create note arguments
    const noteArgs = [new Felt(BigInt(9)), new Felt(BigInt(12))];
    const feltArray = new FeltArray();
    noteArgs.forEach(felt => feltArray.append(felt));

    // Create custom note script
    const noteScript = `
        # Your custom note script here
        # This can include custom validation logic, asset transfers, etc.
    `;

    // Create transaction script
    const transactionScript = new TransactionScript(noteScript);

    // Create output notes array
    const outputNotes = new OutputNotesArray();
    // Add your output notes here

    // Create expected notes array
    const expectedNotes = new NotesArray();
    // Add your expected notes here

    // Build the transaction request
    const transactionRequest = new TransactionRequestBuilder()
        .withCustomScript(transactionScript)
        .withOwnOutputNotes(outputNotes)
        .withExpectedOutputNotes(expectedNotes)
        .build();

    // Create and submit the transaction
    const transactionResult = await webClient.newTransaction(
        accountId,
        transactionRequest
    );

    await webClient.submitTransaction(transactionResult);
    
    // Access transaction details
    console.log("Block number:", transactionResult.blockNum());
    console.log("Created notes:", transactionResult.createdNotes());
    console.log("Consumed notes:", transactionResult.consumedNotes());
    console.log("Account delta:", transactionResult.accountDelta());
} catch (error) {
    console.error("Custom transaction failed:", error.message);
}
```

> **Note**: Custom transactions require a good understanding of the Miden VM and its instruction set. They are powerful but should be used with caution as they can affect the security and correctness of your application.

## Relevant Documentation

For more detailed information about transaction functionality, refer to the following API documentation:

- [WebClient](docs/src/web-client/api/classes/WebClient.md) - Main client class for transaction operations
- [TransactionRequest](docs/src/web-client/api/classes/TransactionRequest.md) - Class representing transaction requests
- [TransactionRequestBuilder](docs/src/web-client/api/classes/TransactionRequestBuilder.md) - Builder class for creating transaction requests
- [TransactionResult](docs/src/web-client/api/classes/TransactionResult.md) - Class representing transaction execution results
- [TransactionProver](docs/src/web-client/api/classes/TransactionProver.md) - Class for transaction proving
- [TransactionScript](docs/src/web-client/api/classes/TransactionScript.md) - Class for defining transaction scripts
- [NoteType](docs/src/web-client/api/enumerations/NoteType.md) - Enumeration for note types (Private/Public)
- [NoteAssets](docs/src/web-client/api/classes/NoteAssets.md) - Class for defining note assets
- [NoteMetadata](docs/src/web-client/api/classes/NoteMetadata.md) - Class for defining note metadata
- [FungibleAsset](docs/src/web-client/api/classes/FungibleAsset.md) - Class for defining fungible assets
- [Felt](docs/src/web-client/api/classes/Felt.md) - Class for working with field elements
- [FeltArray](docs/src/web-client/api/classes/FeltArray.md) - Class for working with arrays of field elements
- [NoteTag](docs/src/web-client/api/classes/NoteTag.md) - Class for defining note tags
- [NoteExecutionMode](docs/src/web-client/api/classes/NoteExecutionMode.md) - Class for defining note execution modes
- [NoteExecutionHint](docs/src/web-client/api/classes/NoteExecutionHint.md) - Class for defining note execution hints
- [OutputNotesArray](docs/src/web-client/api/classes/OutputNotesArray.md) - Class for working with arrays of output notes
- [NotesArray](docs/src/web-client/api/classes/NotesArray.md) - Class for working with arrays of notes

For a complete list of available classes and utilities, see the [SDK API Reference](docs/src/web-client/api/README.md). 