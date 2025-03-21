To use the Miden client library in a Rust project, include it as a dependency. 

In your project's `Cargo.toml`, add:

```toml
miden-client = { version = "0.8" }
```

### Features

The Miden client library supports the [`concurrent`](https://github.com/0xPolygonMiden/miden-client/blob/main/docs/install-and-run.md#concurrent-feature) feature which is recommended for developing applications with the client. To use it, add the following to your project's `Cargo.toml`:

```toml
miden-client = { version = "0.8", features = ["concurrent"] }
```

The library also supports several other features. Please refer to the crate's documentation to learn more.

## Client instantiation

Spin up a client using the following Rust code and supplying a store and RPC endpoint. 

```rust
let sqlite_store = SqliteStore::new("path/to/store".try_into()?).await?;
let store = Arc::new(sqlite_store);

// Generate a random seed for the RpoRandomCoin.
let mut rng = rand::rng();
let coin_seed: [u64; 4] = rng.random();

// Initialize the random coin using the generated seed.
let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));

// Create a store authenticator with the store and random coin.
let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);

// Instantiate the client using a Tonic RPC client
let endpoint = Endpoint::new("https".into(), "localhost".into(), Some(57291));
let client: Client<RpoRandomCoin> = Client::new(
    Box::new(TonicRpcClient::new(&endpoint, 10_000)),
    rng,
    store,
    Arc::new(authenticator),
    false, // Set to true for debug mode, if needed.
);
```

## Create local account

With the Miden client, you can create and track any number of public and local accounts. For local accounts, the state is tracked locally, and the rollup only keeps commitments to the data, which in turn guarantees privacy.

The `AccountBuilder` can be used to create a new account with the specified parameters and components. The following code creates a new local account:

```rust
let key_pair = SecretKey::with_rng(client.rng());
let anchor_block = client.get_latest_epoch_block().await.unwrap();

let (new_account, seed) = AccountBuilder::new(init_seed) // Seed should be random for each account
    .anchor((&anchor_block).try_into().unwrap())
    .account_type(AccountType::RegularAccountImmutableCode)
    .storage_mode(AccountStorageMode::Private)
    .with_component(RpoFalcon512::new(key_pair.public_key()))
    .with_component(BasicWallet)
    .build()?;

client.add_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair), false).await?;
```
Once an account is created, it is kept locally and its state is automatically tracked by the client.

To create an public account, you can specify `AccountStorageMode::Public` like so:

```Rust
let key_pair = SecretKey::with_rng(client.rng());
let anchor_block = client.get_latest_epoch_block().await.unwrap();

let (new_account, seed) = AccountBuilder::new(init_seed) // Seed should be random for each account
    .anchor((&anchor_block).try_into().unwrap())
    .account_type(AccountType::RegularAccountImmutableCode)
    .storage_mode(AccountStorageMode::Public)
    .with_component(RpoFalcon512::new(key_pair.public_key()))
    .with_component(BasicWallet)
    .build()?;

client.add_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair), false).await?;
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
    vec![fungible_asset.into()],
    sender_account_id,
    target_account_id,
);

let transaction_request = TransactionRequestBuilder::pay_to_id(
    payment_transaction,
    None,
    NoteType::Private,
    client.rng(),
)?;

// Execute transaction. No information is tracked after this.
let transaction_execution_result = client.new_transaction(sender_account_id, transaction_request.clone()).await?;

// Prove and submit the transaction, which is stored alongside created notes (if any)
client.submit_transaction(transaction_execution_result).await?
```

You can decide whether you want the note details to be public or private through the `note_type` parameter.
You may also execute a transaction by manually defining a `TransactionRequest` instance. This allows you to run custom code, with custom note arguments as well.
