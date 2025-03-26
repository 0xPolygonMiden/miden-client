# @demox-labs/miden-sdk
The @demox-labs/miden-sdk is a toolkit designed for interacting with the Miden virtual machine via web browser. It offers essential tools and functionalities for developers aiming to integrate or utilize Miden VM capabilities in their web applications.

## Installation

### Stable Version
To install the latest stable version of the package via npm, run:

```javascript
npm i @demox-labs/miden-sdk
```

Or using Yarn:
```javascript
yarn add @demox-labs/miden-sdk
```

### Pre-release ("next") Version
A non-stable version is also maintained. To install the pre-release version, run:

```javascript
npm i @demox-labs/miden-sdk@next
```

Or with Yarn:
```javascript
yarn add @demox-labs/miden-sdk@next
```

> **Note:** The next version must be used in conjunction with a locally running Miden node (instructions can be found [here](https://github.com/0xPolygonMiden/miden-node/tree/next)). Additionally, if you plan to leverage delegated proving in your application, you may need to run a local prover (see [Proving Service instructions](https://github.com/0xPolygonMiden/miden-base/tree/next/bin/proving-service)).

## Usage

The following are just a few simple examples to get started. For more details, see the [API Reference](./docs/README.md).

### Create a New Wallet

```typescript
import { 
    AccountStorageMode,
    WebClient
} from "@demox-labs/miden-sdk";

// Instantiate web client object
const webClient = await WebClient.createClient()

// Set up newWallet params
const accountStorageMode = AccountStorageMode.private();
const mutable = true;

// Create new wallet
const account = await webClient.newWallet(accountStorageMode, mutable);

console.log(account.id().toString()); // account id as hex
console.log(account.isPublic()); // false
console.log(account.isFaucet()); // false
```

### Create a New Faucet

```typescript // Set up newFaucet params
const faucetAccountStorageMode = AccountStorageMode.public();
const nonFungible = false;
const tokenSymbol = "MID";
const decimals = 8;
const maxSupply = BigInt(10000000);

// Create new faucet
const faucet = await webClient.newFaucet(
    faucetAccountStorageMode,
    nonFungible,
    tokenSymbol,
    decimals,
    maxSupply
)

console.log(faucet.id().toString()); // faucet id as hex
console.log(faucet.isPublic()); // true
console.log(faucet.isFaucet()); // true
```

### Create and Execute a New Consume Transaction

Using https://faucet.testnet.miden.io/, send some public test tokens using the account id logged during the new wallet creation. Consume these tokens like this:

```typescript
// Once the faucet finishes minting the tokens, you need to call syncState() so the client knows there is a note available to be consumed. In an actual application, this may need to be in a loop to constantly discover claimable notes.
await webClient.syncState();

// Query the client for consumable notes, and retrieve the id of the new note to be consumed
let consumableNotes = await webClient.getConsumableNotes(account);
const noteIdToConsume = consumableNotes[0].inputNoteRecord().id();

// Create a consume transaction request object
const consumeTransactionRequest = webClient.newConsumeTransactionRequest([
    noteIdToConsume,
]);

// Execute and prove the transaction client side
const consumeTransactionResult = await webClient.newTransaction(
    account,
    consumeTransactionRequest
);

// Submit the transaction to the node
await webClient.submitTransaction(consumeTransactionResult);

// Need to sync state again (in a loop) until the node verifies the transaction
await syncState()

// Check new account balance
const accountBalance = account.vault().getBalance(/* id of remote faucet */).toString();
console.log(accountBalance);
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
