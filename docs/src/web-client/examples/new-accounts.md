# Creating Accounts with the Miden SDK

This guide demonstrates how to create and work with different types of accounts using the Miden SDK. We'll cover initializing the client, creating regular wallet accounts, and setting up faucet accounts. Each example includes detailed annotations to explain the key parameters and returned properties.

## Creating a Regular Wallet Account

Here's how to create a new wallet account with various configuration options:

```typescript
import { AccountStorageMode, WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Set up account parameters
    const accountStorageMode = AccountStorageMode.private(); // Can be private() or public()
    const mutable = true; // Whether the account code can be updated later

    // Create new wallet account
    const account = await webClient.newWallet(accountStorageMode, mutable);

    // Access account properties
    console.log(account.id().toString());        // The account's unique identifier (hex string)
    console.log(account.nonce().toString());     // Current account nonce (starts at 0)
    console.log(account.isPublic());            // Whether the account is public (false for private storage)
    console.log(account.isUpdatable());         // Whether the account code can be updated (true if mutable)
    console.log(account.isFaucet());           // Whether the account is a faucet (false for regular wallets)
    console.log(account.isRegularAccount());    // Whether it's a regular account (true for wallets)
} catch (error) {
    console.error("Failed to create and store new wallet account:", error.message);
}
```

## Creating a Faucet Account

For creating a token faucet, use the following approach:

```typescript
import { AccountStorageMode, WebClient } from "@demox-labs/miden-sdk";

try {
    // Initialize the web client
    const webClient = await WebClient.createClient();

    // Set up faucet parameters
    const faucetStorageMode = AccountStorageMode.public(); // Faucets are typically public
    const nonFungible = false;                 // Whether this is a non-fungible faucet
    const tokenSymbol = "TEST";                // The token symbol (e.g., "TEST", "BTC", etc.)
    const decimals = 8;                        // Number of decimal places for the token
    const maxSupply = BigInt(10000000);        // Maximum supply of tokens that can be minted

    // Create new faucet account
    const faucet = await webClient.newFaucet(
        faucetStorageMode,
        nonFungible,
        tokenSymbol,
        decimals,
        maxSupply
    );

    // Access faucet properties
    console.log(faucet.id().toString());       // The faucet's unique identifier
    console.log(faucet.nonce().toString());    // Current faucet nonce (starts at 0)
    console.log(faucet.isPublic());           // Whether the faucet is public (typically true)
    console.log(faucet.isUpdatable());        // Whether the faucet code can be updated (always false)
    console.log(faucet.isFaucet());          // Whether the account is a faucet (true)
    console.log(faucet.isRegularAccount());   // Whether it's a regular account (false for faucets)
} catch (error) {
    console.error("Failed to create and store new faucet account:", error.message);
}
```

## Relevant Documentation

For more detailed information about the classes and methods used in these examples, refer to the following API documentation:

- [WebClient](docs/src/web-client/api/classes/WebClient.md) - Main client class for interacting with the Miden network
- [Account](docs/src/web-client/api/classes/Account.md) - Account class and its properties
- [AccountStorageMode](docs/src/web-client/api/classes/AccountStorageMode.md) - Storage mode options and methods
- [AccountId](docs/src/web-client/api/classes/AccountId.md) - Account identifier class and utilities
- [AccountCode](docs/src/web-client/api/classes/AccountCode.md) - Account code management
- [AccountStorage](docs/src/web-client/api/classes/AccountStorage.md) - Account storage operations

For a complete list of available classes and utilities, see the [SDK API Reference](docs/src/web-client/api/README.md).
