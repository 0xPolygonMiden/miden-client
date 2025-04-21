# Importing Data with the Miden SDK

This guide demonstrates how to import accounts, notes, and store data using the Miden SDK. We'll cover different ways to import data that was previously exported.

## Importing Accounts

### Importing an Account from Bytes

To import an account that was previously exported:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // accountBytes should be the result of a previous account export
    const result = await webClient.importAccount(accountBytes);
    console.log("Account import result:", result);
} catch (error) {
    console.error("Failed to import account:", error.message);
}
```

### Importing a Public Account from Seed

To import a public account using an initialization seed:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // initSeed should be a Uint8Array containing the initialization seed
    const account = await webClient.importPublicAccountFromSeed(initSeed, true); // true for mutable account
    console.log("Imported account ID:", account.id().toString());
} catch (error) {
    console.error("Failed to import public account:", error.message);
}
```

## Importing Notes

### Note Import Types

When importing notes, there are three types of note files that can be used:

1. **ID Note File**: Contains only the note ID and metadata. This is the most basic form and is useful when you only need to reference a note by its ID.

2. **Full Note File**: Contains the complete note data including the note ID, metadata, and the actual note content. This is used when you need to fully reconstruct the note.

3. **Partial Note File**: Contains the note ID, metadata, and a partial representation of the note content. This is useful when you need to verify a note's existence without having the full content.

To import a note that was previously exported:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // noteBytes should be the result of a previous note export
    const noteId = await webClient.importNote(noteBytes);
    console.log("Imported note ID:", noteId);
} catch (error) {
    console.error("Failed to import note:", error.message);
}
```

## Importing Store Data

To import an entire store (this is a destructive operation that will overwrite the current store):

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // storeDump should be the result of a previous store export
    const result = await webClient.forceImportStore(storeDump);
    console.log("Store import result:", result);
} catch (error) {
    console.error("Failed to import store:", error.message);
}
```

> **Warning**: The `forceImportStore` method is a destructive operation that will completely overwrite the current store. Use with caution and ensure you have a backup if needed.

## Relevant Documentation

For more detailed information about the import functionality, refer to the following API documentation:

- [WebClient](docs/src/web-client/api/classes/WebClient.md) - Main client class for interacting with the Miden network
- [Account](docs/src/web-client/api/classes/Account.md) - Account class returned by importPublicAccountFromSeed

For a complete list of available classes and utilities, see the [SDK API Reference](docs/src/web-client/api/README.md). 