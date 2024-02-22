# How to use the library

In order to use the Miden Client library in a Rust project, you need to include it as a dependency. In your project's `cargo.toml`, add:

````toml
miden_client = { package = "miden-client", git = "https://github.com/0xPolygonMiden/miden-client", branch = "main" }
````

## Features

### Local account creation

With the Miden Client, you can create and track local (not on-chain) accounts. What this means is that all state is tracked locally, and the rollup only keeps commitments to the data, which in turn guarantees privacy:

```Rust
    let account_template = AccountTemplate::BasicWallet {
        mutable_code: false,
        storage_mode: accounts::AccountStorageMode::Local,
    };
    
    let (new_account, account_seed) = client.new_account(client_template)?;
```

The `AccountTemplate` enum defines which type of account will be created. Note that once an account is created, it will be kept locally and its state will automatically be tracked by the client. 