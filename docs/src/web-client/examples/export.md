# Exporting Data with the Miden SDK

This guide demonstrates how to export notes and store data using the Miden SDK. We'll cover different ways to export notes with varying levels of detail and how to export the entire store.

## Exporting Notes

You can export a note with different levels of detail using the `exportNote` method:

```typescript
try {
    // Export a note with just its ID
    const noteIdExport = await webClient.exportNote("0x1234...", "Id");
    console.log("Note ID Export:", noteIdExport);

    // Export a note with full details including inclusion proof
    const fullNoteExport = await webClient.exportNote("0x1234...", "Full");
    console.log("Full Note Export:", fullNoteExport);

    // Export a note with partial details (default if type is not specified)
    const partialNoteExport = await webClient.exportNote("0x1234...", "Partial");
    console.log("Partial Note Export:", partialNoteExport);
} catch (error) {
    console.error("Failed to export note:", error.message);
}
```

The export types are:
- `Id`: Exports only the note ID
- `Full`: Exports the complete note with its inclusion proof (requires the note to have an inclusion proof)
- `Partial`: Exports note details including metadata and the block number after which it was created

## Exporting the Store

To export the entire store:

```typescript
try {
    const storeExport = await webClient.exportStore();
    console.log("Store Export:", storeExport);
} catch (error) {
    console.error("Failed to export store:", error.message);
}
```

The store export contains all the data managed by the client, which can be useful for backup or migration purposes.

## Relevant Documentation

For more detailed information about the export functionality, refer to the following API documentation:

- [WebClient](docs/src/web-client/api/classes/WebClient.md) - Main client class for interacting with the Miden network

For a complete list of available classes and utilities, see the [SDK API Reference](docs/src/web-client/api/README.md). 