# Working with Note Tags in the Miden SDK

Note tags are used to specify how notes should be executed and who can consume them. They also serve as a fuzzy filter mechanism for retrieving note updates during sync operations.

## Basic Tag Operations

### Adding a Tag

To add a tag for the client to track:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Add a tag
    await webClient.addTag("123"); // Tag must be a valid u32 number passed as a string
    console.log("Tag added successfully");
} catch (error) {
    console.error("Failed to add tag:", error.message);
}
```

### Removing a Tag

To remove a tag that was previously added:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Remove a tag
    await webClient.removeTag("123"); // Tag must be a valid u32 number passed as a string
    console.log("Tag removed successfully");
} catch (error) {
    console.error("Failed to remove tag:", error.message);
}
```

### Listing Tags

To get all tags currently being tracked:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Get all tags
    const tags = await webClient.listTags();
    console.log("Current tags:", tags);
} catch (error) {
    console.error("Failed to list tags:", error.message);
}
```

## Tag Sources and Sync Behavior

Tags can come from different sources, which affects how they are used during sync operations:

1. **Account Tags**: Automatically added for accounts being tracked by the client. These tags ensure that notes directed to tracked accounts are retrieved during sync.

2. **Note Tags**: Automatically added for expected notes being tracked. These tags help retrieve updates for specific notes.

3. **User Tags**: Manually added by the user. These tags act as a fuzzy filter to retrieve notes that match the tag pattern during sync.

During sync operations, the client uses these tags to:
- Retrieve note-related information for notes with matching tags
- Track updates for notes directed to managed accounts
- Monitor changes to expected notes
- Filter notes based on user-defined criteria

## Managing Multiple Tags

You can add and remove multiple tags in a loop:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Add multiple tags
    const tagsToAdd = ["123", "456", "789"];
    for (const tag of tagsToAdd) {
        await webClient.addTag(tag);
    }

    // Remove multiple tags
    const tagsToRemove = ["123", "456"];
    for (const tag of tagsToRemove) {
        await webClient.removeTag(tag);
    }
} catch (error) {
    console.error("Failed to manage tags:", error.message);
}
```

## Important Notes

- Tags must be valid `u32` numbers passed as strings
- Tags for managed accounts are handled automatically by the client
- User-added tags can be removed, but system-generated tags (for accounts and notes) cannot
- Tags are used as a fuzzy filter during sync operations to retrieve relevant note updates

## Relevant Documentation

For more detailed information about tag functionality, refer to the following API documentation:

- [WebClient](docs/src/web-client/api/classes/WebClient.md) - Main client class for tag operations
- [NoteTag](docs/src/web-client/api/classes/NoteTag.md) - Class for working with note tags
- [AccountId](docs/src/web-client/api/classes/AccountId.md) - Class for working with account IDs

For a complete list of available classes and utilities, see the [SDK API Reference](docs/src/web-client/api/README.md). 