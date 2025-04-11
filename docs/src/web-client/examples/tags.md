# Working with Tags in the Miden SDK

This guide demonstrates how to work with tags in the Miden SDK. Tags are used to categorize and organize notes in the Miden network.

## Basic Tag Operations

### Adding a Tag

To add a new tag to the system:

```typescript
try {
    // Add a tag (must be a valid u32 number)
    const tag = "123";  // Tag value as a string
    await webClient.addTag(tag);
    
    console.log("Tag added successfully");
} catch (error) {
    console.error("Failed to add tag:", error.message);
}
```

### Removing a Tag

To remove a tag from the system:

```typescript
try {
    // Remove a tag (must be a valid u32 number)
    const tag = "123";  // Tag value as a string
    await webClient.removeTag(tag);
    
    console.log("Tag removed successfully");
} catch (error) {
    console.error("Failed to remove tag:", error.message);
}
```

### Listing All Tags

To get a list of all tags in the system:

```typescript
try {
    // Get all tags
    const tags = await webClient.listTags();
    console.log("Current tags:", tags);
} catch (error) {
    console.error("Failed to list tags:", error.message);
}
```

## Managing Multiple Tags

Here's an example of managing multiple tags:

```typescript
try {
    // Add multiple tags
    const tagsToAdd = ["123", "456", "789"];
    for (const tag of tagsToAdd) {
        await webClient.addTag(tag);
    }
    
    // List all tags
    const allTags = await webClient.listTags();
    console.log("All tags after adding:", allTags);
    
    // Remove some tags
    const tagsToRemove = ["123", "789"];
    for (const tag of tagsToRemove) {
        await webClient.removeTag(tag);
    }
    
    // List remaining tags
    const remainingTags = await webClient.listTags();
    console.log("Remaining tags:", remainingTags);
} catch (error) {
    console.error("Failed to manage tags:", error.message);
}
```

## Important Notes

- Tags must be valid u32 numbers (passed as strings)
- Tags are used to categorize and organize notes
- The same tag can be used for multiple notes
- Tags are stored locally in the client's state

## Relevant Documentation

For more detailed information about tag functionality, refer to the following API documentation:

- [WebClient](docs/src/web-client/api/classes/WebClient.md) - Main client class for tag operations

For a complete list of available classes and utilities, see the [SDK API Reference](docs/src/web-client/api/README.md). 