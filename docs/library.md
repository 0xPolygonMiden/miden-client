To use the Miden client library in a Rust project, include it as a dependency. 

In your project's `cargo.toml`, add:

```toml
miden_client = { 
    package = "miden-client", 
    git = "https://github.com/0xPolygonMiden/miden-client", 
    branch = "main" 
}
```

## Client instantiation

Spin up a client using the following Rust code and supplying a store and RPC endpoint. 

The current supported store is the `SqliteDataStore`, which is a SQLite implementation of the incoming `Store` trait.

```rust
let client: Client<TonicRpcClient, SqliteDataStore> = {
    
    let store = Store::new((&client_config).into()).map_err(ClientError::StoreError)?;

    Client::new(
        
        client_config,
        TonicRpcClient::new(&rpc_endpoint),
        SqliteDataStore::new(store),

    )?
};
```

## Create local account

With the Miden client, you can create and track local (not on-chain) accounts. 

This means that all state is tracked locally, and the rollup only keeps commitments to the data, which in turn guarantees privacy.

The following code creates a new account:

```rust
let account_template = AccountTemplate::BasicWallet {
    mutable_code: false,
    storage_mode: accounts::AccountStorageMode::Local,
};
    
let (new_account, account_seed) = client.new_account(client_template)?;
```

The `AccountTemplate` enum defines the type of account. 

Once an account is created, it is kept locally and its state is automatically tracked by the client.

The client can create and store any number of accounts.

## Execute transaction

In order to execute a transaction, you first need to define the transaction type with the `TransactionTemplate` enum. 

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

let transaction_template: TransactionTemplate = TransactionTemplate::P2ID(payment_transaction);

// Execute transaction. No information is tracked after this.
let transaction_execution_result =
        client.new_transaction(transaction_template.clone())?;

// Prove and submit the transaction, which is stored alongside created notes (if any)
client
    .send_transaction(transaction_execution_result)
    .await?
```
