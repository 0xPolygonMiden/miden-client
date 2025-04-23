# Retrieving Accounts with the Miden SDK

This guide demonstrates how to retrieve and work with existing accounts using the Miden SDK. We'll cover getting a single account by ID, listing all accounts, and accessing account properties. Each example includes detailed annotations to explain the key parameters and returned properties.

## Retrieving a Single Account

To retrieve a specific account by its ID:

```typescript
import { AccountId, WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Create an AccountId from a hex string
    const accountId = AccountId.fromHex("0x1234..."); // Replace with actual account ID

    // Get the account
    const account = await webClient.getAccount(accountId);

    if (!account){
        console.log("Account not found");
        return;
    }

    // Access account properties
    console.log(account.id().toString());        // The account's unique identifier
    console.log(account.nonce().toString());     // Current account nonce
    console.log(account.commitment().toHex());   // Account commitment hash
    console.log(account.isPublic());            // Whether the account is public
    console.log(account.isUpdatable());         // Whether the account code can be updated
    console.log(account.isFaucet());           // Whether the account is a faucet
    console.log(account.isRegularAccount());    // Whether it's a regular account
} catch (error) {
    console.error("Failed to retrieve account:", error.message);
}
```

## Listing All Accounts

To retrieve a list of all accounts tracked by the client:

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Get all account headers
    const accounts = await webClient.getAccounts();

    // Iterate through accounts
    for (const account of accounts) {
        console.log(account.id().toString());           // Account ID
        console.log(account.nonce().toString());        // Account nonce
        console.log(account.commitment().toHex());      // Account commitment
        console.log(account.vaultCommitment().toHex()); // Vault commitment
        console.log(account.storageCommitment().toHex()); // Storage commitment
        console.log(account.codeCommitment().toHex());  // Code commitment
    }
} catch (error) {
    console.error("Failed to retrieve accounts:", error.message);
}
```

## Relevant Documentation

For more detailed information about the classes and methods used in these examples, refer to the following API documentation:

- [WebClient](docs/src/web-client/api/classes/WebClient.md) - Main client class for interacting with the Miden network
- [Account](docs/src/web-client/api/classes/Account.md) - Account class and its properties
- [AccountId](docs/src/web-client/api/classes/AccountId.md) - Account identifier class and utilities
- [RpoDigest](docs/src/web-client/api/classes/RpoDigest.md) - Commitment hash class used for various account properties

For a complete list of available classes and utilities, see the [SDK API Reference](docs/src/web-client/api/README.md). 