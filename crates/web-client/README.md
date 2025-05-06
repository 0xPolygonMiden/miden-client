# @demox-labs/miden-sdk

## Overview
The `@demox-labs/miden-sdk` is a comprehensive software development toolkit (SDK) for interacting with the Miden blockchain and virtual machine from within a web application. It provides developers with everything needed to:

* Interact with the Miden chain (e.g. syncing accounts, submitting transactions)
* Create and manage Miden transactions
* Run the Miden VM to execute programs
* Generate zero-knowledge proofs using the Miden Prover (with support for delegated proving)
* Integrate Miden capabilities seamlessly into browser-based environments

Whether you're building a wallet, dApp, or other blockchain-integrated application, this SDK provides the core functionality to bridge your frontend with Miden's powerful ZK architecture.

> **Note:** This README provides a high-level overview of the web client SDK.
For more detailed documentation, API references, and usage examples, see the documentation [here](../../docs/src/web-client) (TBD).

### SDK Structure and Build Process

This SDK is published as an NPM package, built from the `web-client` crate. The `web-client` crate is a Rust crate targeting WebAssembly (WASM), and it uses `wasm-bindgen` to generate JavaScript bindings. It depends on the lower-level `rust-client` crate, which implements the core functionality for interacting with the Miden chain.

Both a `Cargo.toml` and a `package.json` are present in the `web-client` directory to support Rust compilation and NPM packaging respectively.

The build process is powered by a custom `rollup.config.js` file, which orchestrates three main steps:

1. __WASM Module Build__: Compiles the `web-client` Rust crate into a WASM module using `@wasm-tool/rollup-plugin-rust`, enabling WebAssembly features such as atomics and bulk memory operations.

2. __Worker Build__: Bundles a dedicated web worker file that enables off-main-thread execution for computationally intensive functions.

3. __Main Entry Point Build__: Bundles the top-level JavaScript module (`index.js`) which serves as the main API surface for consumers of the SDK.

This setup allows the SDK to be seamlessly consumed in JavaScript environments, particularly in web applications.

## Installation

### Stable Version
A non-stable version of the SDK is also maintained, which tracks the `next` branch of the Miden client repository (essentially the development branch). To install the pre-release version, run:

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

> **Note:** The `next` version of the SDK must be used in conjunction with a locally running Miden node built from the `next` branch of the `miden-node` repository. This is necessary because the public testnet runs the stable `main` branch, which may not be compatible with the latest development features in `next`. Instructions to run a local node can be found [here](https://github.com/0xMiden/miden-node/tree/next) on the `next` branch of the `miden-node` repository. Additionally, if you plan to leverage delegated proving in your application, you may need to run a local prover (see [Proving Service instructions](https://github.com/0xMiden/miden-base/tree/next/bin/proving-service)).

## Building and Testing the Web Client

If you're interested in contributing to the web client and need to build it locally, you can do so via:

```
yarn install
yarn build
```

This will:
* Install all JavaScript dependencies,
* Compile the Rust code to WebAssembly,
* Generate the JavaScript bindings via wasm-bindgen,
* And bundle the SDK into the dist/ directory using Rollup.

To run integration tests after building, use: 
```
yarn test
```

This runs a suite of integration tests to verify the SDK’s functionality in a web context.

## Usage

The following are just a few simple examples to get started. For more details, see the [API Reference](../../docs/src/web-client/api).

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
