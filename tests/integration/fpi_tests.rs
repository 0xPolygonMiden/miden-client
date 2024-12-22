use miden_client::{
    accounts::{Account, AccountData, AccountTemplate, StorageSlot},
    testing::prepare_word,
    transactions::{TransactionKernel, TransactionRequestBuilder},
    Felt, Word,
};
use miden_lib::accounts::auth::RpoFalcon512;
use miden_objects::{
    accounts::{AccountBuilder, AccountComponent, AccountStorageMode, AuthSecretKey, StorageMap},
    crypto::dsa::rpo_falcon512::SecretKey,
    transaction::TransactionScript,
    Digest, FieldElement,
};

use super::common::*;

// FPI TESTS
// ================================================================================================

const FPI_STORAGE_VALUE: Word =
    [Felt::new(9u64), Felt::new(12u64), Felt::new(18u64), Felt::new(30u64)];

#[tokio::test]
async fn test_standard_fpi_public() {
    test_standard_fpi(AccountStorageMode::Public).await;
}

#[tokio::test]
async fn test_standard_fpi_private() {
    test_standard_fpi(AccountStorageMode::Private).await;
}

/// Tests the standard FPI functionality for the given storage mode.
///
/// This function sets up a foreign account with a custom component that retrieves a value from its
/// storage. It then deploys the foreign account and creates a native account to execute a
/// transaction that calls the foreign account's procedure via FPI. The test also verifies that the
/// foreign account's code is correctly cached after the transaction.
async fn test_standard_fpi(storage_mode: AccountStorageMode) {
    let mut client = create_test_client().await;
    wait_for_node(&mut client).await;

    let (foreign_account, foreign_seed, secret_key, proc_root) = foreign_account(storage_mode);
    let foreign_account_id = foreign_account.id();

    client
        .import_account(
            AccountData::new(
                foreign_account,
                Some(foreign_seed),
                AuthSecretKey::RpoFalcon512(secret_key.clone()),
            ),
            false,
        )
        .await
        .unwrap();

    let deployment_tx_script = TransactionScript::compile(
        "begin 
                call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512 
            end",
        vec![],
        TransactionKernel::assembler(),
    )
    .unwrap();

    println!("Deploying foreign account with an auth transaction");

    let tx = client
        .new_transaction(
            foreign_account_id,
            TransactionRequestBuilder::new()
                .with_custom_script(deployment_tx_script)
                .unwrap()
                .build(),
        )
        .await
        .unwrap();
    let tx_id = tx.executed_transaction().id();
    client.submit_transaction(tx).await.unwrap();
    wait_for_tx(&mut client, tx_id).await;

    println!("Calling FPI functions with new account");

    let (native_account, _native_seed) = client
        .new_account(AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Public,
        })
        .await
        .unwrap();

    let tx_script = format!(
        "
        use.miden::tx
        use.miden::account
        begin
            # push the hash of the `get_fpi_item` account procedure
            push.{proc_root}
    
            # push the foreign account id
            push.{foreign_account_id}
            # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index]
    
            exec.tx::execute_foreign_procedure
            push.{fpi_value} assert_eqw
    
            call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512 
        end
        ",
        fpi_value = prepare_word(&FPI_STORAGE_VALUE)
    );

    let tx_script =
        TransactionScript::compile(tx_script, vec![], TransactionKernel::assembler()).unwrap();
    _ = client.sync_state().await.unwrap();

    // Wait for a couple of blocks to enforce a sync
    _ = wait_for_blocks(&mut client, 2).await;

    // Before the transaction there are no cached foreign accounts
    let foreign_accounts = client
        .test_store()
        .get_foreign_account_code(vec![foreign_account_id])
        .await
        .unwrap();
    assert!(foreign_accounts.is_empty());

    // Create tx request with FPI

    let builder = TransactionRequestBuilder::new().with_custom_script(tx_script).unwrap();
    let tx_request = if storage_mode == AccountStorageMode::Public {
        builder
            .with_public_foreign_accounts([(foreign_account_id, vec![0])])
            .unwrap()
            .build()
    } else {
        // Get foreign account current state (after 1st deployment tx)
        let foreign_account = client.get_account(foreign_account_id).await.unwrap();
        builder
            .with_private_foreign_accounts([foreign_account.account().clone()])
            .unwrap()
            .build()
    };

    let tx_result = client.new_transaction(native_account.id(), tx_request).await.unwrap();

    client.submit_transaction(tx_result).await.unwrap();

    // After the transaction the foreign account should be cached (for public accounts only)
    if storage_mode == AccountStorageMode::Public {
        let foreign_accounts = client
            .test_store()
            .get_foreign_account_code(vec![foreign_account_id])
            .await
            .unwrap();
        assert_eq!(foreign_accounts.len(), 1);
    }
}

/// Builds a foreign account with a custom component that retrieves a value from its storage (map).
///
/// # Returns
///
/// A tuple containing:
/// - `Account` - The constructed foreign account.
/// - `Word` - The seed used to initialize the account.
/// - `SecretKey` - The secret key associated with the account's authentication component.
/// - `Digest` - The procedure root of the custom component's procedure.
pub fn foreign_account(storage_mode: AccountStorageMode) -> (Account, Word, SecretKey, Digest) {
    // store our expected value on map from slot 0 (map key 15)
    let mut storage_map = StorageMap::new();
    storage_map
        .insert([Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::new(15)].into(), FPI_STORAGE_VALUE);

    let get_item_component = AccountComponent::compile(
        "
        export.get_fpi_map_item
            # map key
            push.15
            # item index
            push.0
            exec.::miden::account::get_map_item
            swapw dropw
        end
        ",
        TransactionKernel::assembler(),
        vec![StorageSlot::Map(storage_map)],
    )
    .unwrap()
    .with_supports_all_types();

    let secret_key = SecretKey::new();
    let auth_component = RpoFalcon512::new(secret_key.public_key());

    let (account, seed) = AccountBuilder::new()
        .init_seed(Default::default())
        .with_component(get_item_component.clone())
        .with_component(auth_component)
        .storage_mode(storage_mode)
        .build()
        .unwrap();

    let proc_root = get_item_component.mast_forest().procedure_digests().next().unwrap();
    (account, seed, secret_key, proc_root)
}
