## Miden Web SDK (@demox-labs/miden-sdk)

The `@demox-labs/miden-sdk` is a browser-focused toolkit designed to enable full-featured interaction with the Miden blockchain directly from web applications. It wraps core functionality provided by the Miden Rust client and compiles to WebAssembly (WASM) with TypeScript bindings, making it ideal for use in wallets, dApps, or browser-based dev tools.

## Capabilities
The SDK provides APIs to:

* Interact with the Miden chain (e.g., syncing accounts, submitting transactions)
* Build and manage Miden transactions
* Execute programs in the Miden Virtual Machine (VM)
* Generate zero-knowledge proofs via the Miden Prover
* Support delegated proving setups
* Run entirely in the browser using WASM and web workers

## Architecture
The SDK is built from the `web-client` crate, which:

* Is implemented in Rust and compiled to WebAssembly
* Uses `wasm-bindgen` to expose JavaScript-compatible bindings
* Depends on the rust-client crate, which contains core logic for blockchain interaction

A custom `rollup.config.js` bundles the WASM module, JS bindings, and web worker into a distributable NPM package.

## Installation & Usage
The SDK is published to NPM and can be installed via:

```
npm install @demox-labs/miden-sdk
# or
yarn add @demox-labs/miden-sdk
```

See the [README](https://github.com/0xPolygonMiden/miden-client/blob/main/crates/web-client/README.md) for full installation instructions and some usage instructions, including code examples for wallet creation, transaction execution, and syncing state.
