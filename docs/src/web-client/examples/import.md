# Importing Data with the Miden SDK

This guide demonstrates how to import accounts, notes, and store data using the Miden SDK. We'll cover different ways to import data that was previously exported.

## Importing Accounts

### Importing an Account from Bytes

To import an account that was previously exported:

```typescript
try {
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
try {
    // initSeed should be a Uint8Array containing the initialization seed
    const account = await webClient.importPublicAccountFromSeed(initSeed, true); // true for mutable account
    console.log("Imported account ID:", account.id().toString());
} catch (error) {
    console.error("Failed to import public account:", error.message);
}
```

## Importing Notes

To import a note that was previously exported:

```typescript
try {
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
try {
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