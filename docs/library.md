---
comments: true
---

To use the Miden client library in a Rust project, include it as a dependency. 

In your project's `Cargo.toml`, add:

```toml
miden-client = { version = "0.5" }
```

### Features

The Miden client library supports the [`testing`](https://github.com/0xPolygonMiden/miden-client/blob/main/docs/install-and-run.md#testing-feature) and [`concurrent`](https://github.com/0xPolygonMiden/miden-client/blob/main/docs/install-and-run.md#concurrent-feature) features which are both recommended for developing applications with the client. To use them, add the following to your project's `Cargo.toml`:

```toml
miden-client = { version = "0.5", features = ["testing", "concurrent"] }
```

## Client instantiation

Spin up a client using the following Rust code and supplying a store and RPC endpoint. 

The current supported store is the `SqliteDataStore`, which is a SQLite implementation of the `Store` trait.

```rust
let client: Client<TonicRpcClient, SqliteDataStore> = {
    
    let store = SqliteStore::new((&client_config).into()).map_err(ClientError::StoreError)?;
    let store = Rc::new(store);

    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);

    let client = Client::new(
        TonicRpcClient::new(&client_config.rpc),
        rng,
        store,
        authenticator,
        false, // set to true if you want a client with debug mode
    )
};
```

## Create local account

With the Miden client, you can create and track any number of public and local accounts. For local accounts, the state is tracked locally, and the rollup only keeps commitments to the data, which in turn guarantees privacy.

The `AccountTemplate` enum defines the type of account. The following code creates a new local account:

```rust
let account_template = AccountTemplate::BasicWallet {
    mutable_code: false,
    storage_mode: AccountStorageMode::Local,
};
    
let (new_account, account_seed) = client.new_account(account_template)?;
```
Once an account is created, it is kept locally and its state is automatically tracked by the client.

To create an public account, you can specify `AccountStorageMode::Public` like so:

```Rust
let account_template = AccountTemplate::BasicWallet {
    mutable_code: false,
    storage_mode: AccountStorageMode::Public,
};

let (new_account, account_seed) = client.new_account(client_template)?;
```

The account's state is also tracked locally, but during sync the client updates the account state by querying the node for the most recent account data.

## Execute transaction

In order to execute a transaction, you first need to define which type of transaction is to be executed. This may be done with the `TransactionRequest` which represents a general definition of a transaction. Some standardized constructors are available for common transaction types.

Here is an example for a `pay-to-id` transaction type:

```rust
// Define asset
let faucet_id = AccountId::from_hex(faucet_id)?;
let fungible_asset = FungibleAsset::new(faucet_id, *amount)?.into();

let sender_account_id = AccountId::from_hex(bob_account_id)?;
let target_account_id = AccountId::from_hex(alice_account_id)?;
let payment_transaction = PaymentTransactionData::new(
    fungible_asset,
    sender_account_id,
    target_account_id,
);

let transaction_request = TransactionRequest::pay_to_id(
    payment_transaction,
    None,
    NoteType::Private,
    client.rng(),
)?;

// Execute transaction. No information is tracked after this.
let transaction_execution_result = client.new_transaction(sender_account_id, transaction_request.clone())?;

// Prove and submit the transaction, which is stored alongside created notes (if any)
client.send_transaction(transaction_execution_result).await?
```

You can decide whether you want the note details to be public or private through the `note_type` parameter.
You may also execute a transaction by manually defining a `TransactionRequest` instance. This allows you to run custom code, with custom note arguments as well.
