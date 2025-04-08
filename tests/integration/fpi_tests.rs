use miden_client::{
    Felt, Word,
    account::{Account, StorageSlot},
    auth::AuthSecretKey,
    block::BlockHeader,
    rpc::domain::account::{AccountStorageRequirements, StorageMapKey},
    transaction::{
        ForeignAccount, ForeignAccountInputs, TransactionKernel, TransactionRequestBuilder,
    },
};
use miden_lib::{account::auth::RpoFalcon512, utils::word_to_masm_push_string};
use miden_objects::{
    Digest,
    account::{AccountBuilder, AccountComponent, AccountStorageMode, StorageMap},
    crypto::dsa::rpo_falcon512::SecretKey,
    transaction::TransactionScript,
    vm::AdviceInputs,
};

use super::common::*;

// FPI TESTS
// ================================================================================================
const MAP_KEY: [Felt; 4] = [Felt::new(15), Felt::new(15), Felt::new(15), Felt::new(15)];
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

#[tokio::test]
async fn test_fpi_execute_program() {
    let (mut client, mut keystore) = create_test_client().await;
    client.sync_state().await.unwrap();

    // Deploy a foreign account
    let (foreign_account, proc_root) = deploy_foreign_account(
        &mut client,
        &mut keystore,
        AccountStorageMode::Public,
        format!(
            "export.get_fpi_map_item
                # map key
                push.{map_key}
                # item index
                push.0
                exec.::miden::account::get_map_item
                swapw dropw
            end",
            map_key = word_to_masm_push_string(&MAP_KEY)
        ),
    )
    .await
    .unwrap();
    let foreign_account_id = foreign_account.id();

    let (wallet, ..) = insert_new_wallet(&mut client, AccountStorageMode::Private, &keystore)
        .await
        .unwrap();

    let code = format!(
        "
        use.miden::tx
        use.miden::account
        begin
            # push the root of the `get_fpi_item` account procedure
            push.{proc_root}
    
            # push the foreign account id
            push.{account_id_suffix} push.{account_id_prefix}
            # => [foreign_id_prefix, foreign_id_suffix, FOREIGN_PROC_ROOT, storage_item_index]
    
            exec.tx::execute_foreign_procedure
        end
        ",
        account_id_prefix = foreign_account_id.prefix().as_u64(),
        account_id_suffix = foreign_account_id.suffix(),
    );

    let tx_script = client.compile_tx_script(vec![], &code).unwrap();
    _ = client.sync_state().await.unwrap();

    // Wait for a couple of blocks so that the account gets committed
    _ = wait_for_blocks(&mut client, 2).await;

    let storage_requirements =
        AccountStorageRequirements::new([(0u8, &[StorageMapKey::from(MAP_KEY)])]);

    let output_stack = client
        .execute_program(
            wallet.id(),
            tx_script,
            AdviceInputs::default(),
            [ForeignAccount::public(foreign_account_id, storage_requirements).unwrap()].into(),
        )
        .await
        .unwrap();

    let mut expected_stack = [Felt::new(0); 16];
    expected_stack[3] = FPI_STORAGE_VALUE[0];
    expected_stack[2] = FPI_STORAGE_VALUE[1];
    expected_stack[1] = FPI_STORAGE_VALUE[2];
    expected_stack[0] = FPI_STORAGE_VALUE[3];

    assert_eq!(output_stack, expected_stack);
}

#[tokio::test]
async fn test_nested_fpi_calls() {
    let (mut client, mut keystore) = create_test_client().await;
    wait_for_node(&mut client).await;

    let (inner_foreign_account, inner_proc_root) = deploy_foreign_account(
        &mut client,
        &mut keystore,
        AccountStorageMode::Public,
        format!(
            "export.get_fpi_map_item
                # map key
                push.{map_key}
                # item index
                push.0
                exec.::miden::account::get_map_item
                swapw dropw
            end",
            map_key = word_to_masm_push_string(&MAP_KEY)
        ),
    )
    .await
    .unwrap();
    let inner_foreign_account_id = inner_foreign_account.id();

    let (outer_foreign_account, outer_proc_root) = deploy_foreign_account(
        &mut client,
        &mut keystore,
        AccountStorageMode::Public,
        format!(
            "
            use.miden::tx
            export.get_fpi_map_item
                # push the hash of the `get_fpi_item` account procedure
                push.{inner_proc_root}

                # push the foreign account id
                push.{account_id_suffix} push.{account_id_prefix}
                # => [foreign_id_prefix, foreign_id_suffix, FOREIGN_PROC_ROOT, storage_item_index]

                exec.tx::execute_foreign_procedure

                # add one to the result of the foreign procedure call
                add.1
            end
            ",
            account_id_prefix = inner_foreign_account_id.prefix().as_u64(),
            account_id_suffix = inner_foreign_account_id.suffix(),
        ),
    )
    .await
    .unwrap();
    let outer_foreign_account_id = outer_foreign_account.id();

    println!("Calling FPI function inside a FPI function with new account");

    let (native_account, _native_seed, _) =
        insert_new_wallet(&mut client, AccountStorageMode::Public, &keystore)
            .await
            .unwrap();

    let tx_script = format!(
        "
        use.miden::tx
        use.miden::account
        begin
            # push the hash of the `get_fpi_item` account procedure
            push.{outer_proc_root}
    
            # push the foreign account id
            push.{account_id_suffix} push.{account_id_prefix}
            # => [foreign_id_prefix, foreign_id_suffix, FOREIGN_PROC_ROOT, storage_item_index]
    
            exec.tx::execute_foreign_procedure
            push.{fpi_value} add.1 assert_eqw

            call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512 
        end
        ",
        fpi_value = word_to_masm_push_string(&FPI_STORAGE_VALUE),
        account_id_prefix = outer_foreign_account_id.prefix().as_u64(),
        account_id_suffix = outer_foreign_account_id.suffix(),
    );

    let tx_script =
        TransactionScript::compile(tx_script, vec![], TransactionKernel::assembler()).unwrap();
    client.sync_state().await.unwrap();

    // Wait for a couple of blocks so that the account gets committed
    wait_for_blocks(&mut client, 2).await;

    // Create transaction request with FPI
    let builder = TransactionRequestBuilder::new().with_custom_script(tx_script);

    // We will require slot 0, key `MAP_KEY` as well as account proof
    let storage_requirements =
        AccountStorageRequirements::new([(0u8, &[StorageMapKey::from(MAP_KEY)])]);

    let foreign_accounts = [
        ForeignAccount::public(inner_foreign_account_id, storage_requirements.clone()).unwrap(),
        ForeignAccount::public(outer_foreign_account_id, storage_requirements).unwrap(),
    ];

    let tx_request = builder.with_foreign_accounts(foreign_accounts).build().unwrap();
    let tx_result = client.new_transaction(native_account.id(), tx_request).await.unwrap();

    client.submit_transaction(tx_result).await.unwrap();
}

/// Tests the standard FPI functionality for the given storage mode.
///
/// This function sets up a foreign account with a custom component that retrieves a value from its
/// storage. It then deploys the foreign account and creates a native account to execute a
/// transaction that calls the foreign account's procedure via FPI. The test also verifies that the
/// foreign account's code is correctly cached after the transaction.
async fn test_standard_fpi(storage_mode: AccountStorageMode) {
    let (mut client, mut keystore) = create_test_client().await;
    wait_for_node(&mut client).await;

    let (foreign_account, proc_root) = deploy_foreign_account(
        &mut client,
        &mut keystore,
        storage_mode,
        format!(
            "export.get_fpi_map_item
                # map key
                push.{map_key}
                # item index
                push.0
                exec.::miden::account::get_map_item
                swapw dropw
            end",
            map_key = word_to_masm_push_string(&MAP_KEY)
        ),
    )
    .await
    .unwrap();

    let foreign_account_id = foreign_account.id();

    println!("Calling FPI functions with new account");

    let (native_account, _native_seed, _) =
        insert_new_wallet(&mut client, AccountStorageMode::Public, &keystore)
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
            push.{account_id_suffix} push.{account_id_prefix}
            # => [foreign_id_prefix, foreign_id_suffix, FOREIGN_PROC_ROOT, storage_item_index]
    
            exec.tx::execute_foreign_procedure
            push.{fpi_value} assert_eqw
    
            call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512 
        end
        ",
        fpi_value = word_to_masm_push_string(&FPI_STORAGE_VALUE),
        account_id_prefix = foreign_account_id.prefix().as_u64(),
        account_id_suffix = foreign_account_id.suffix(),
    );

    let tx_script =
        TransactionScript::compile(tx_script, vec![], TransactionKernel::assembler()).unwrap();
    _ = client.sync_state().await.unwrap();

    // Wait for a couple of blocks so that the account gets committed
    _ = wait_for_blocks(&mut client, 2).await;

    // Before the transaction there are no cached foreign accounts
    let foreign_accounts = client
        .test_store()
        .get_foreign_account_code(vec![foreign_account_id])
        .await
        .unwrap();
    assert!(foreign_accounts.is_empty());

    // Create transaction request with FPI
    let builder = TransactionRequestBuilder::new().with_custom_script(tx_script);

    // We will require slot 0, key `MAP_KEY` as well as account proof
    let storage_requirements =
        AccountStorageRequirements::new([(0u8, &[StorageMapKey::from(MAP_KEY)])]);

    let foreign_account = if storage_mode == AccountStorageMode::Public {
        ForeignAccount::public(foreign_account_id, storage_requirements)
    } else {
        // Get current foreign account current state from the store (after 1st deployment tx)
        let foreign_account: Account =
            client.get_account(foreign_account_id).await.unwrap().unwrap().into();
        ForeignAccount::private(
            ForeignAccountInputs::from_account(foreign_account, &storage_requirements).unwrap(),
        )
    };

    let tx_request = builder.with_foreign_accounts([foreign_account.unwrap()]).build().unwrap();
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

/// Builds a foreign account with a custom component that exports the specified code.
///
/// # Returns
///
/// A tuple containing:
/// - `Account` - The constructed foreign account.
/// - `Word` - The seed used to initialize the account.
/// - `Digest` - The procedure root of the custom component's procedure.
fn foreign_account_with_code(
    storage_mode: AccountStorageMode,
    anchor_block_header: &BlockHeader,
    code: String,
) -> (Account, Word, Digest, SecretKey) {
    // store our expected value on map from slot 0 (map key 15)
    let mut storage_map = StorageMap::new();
    storage_map.insert(MAP_KEY.into(), FPI_STORAGE_VALUE);

    let get_item_component = AccountComponent::compile(
        code,
        TransactionKernel::assembler(),
        vec![StorageSlot::Map(storage_map)],
    )
    .unwrap()
    .with_supports_all_types();

    let secret_key = SecretKey::new();
    let auth_component = RpoFalcon512::new(secret_key.public_key());

    let (account, seed) = AccountBuilder::new(Default::default())
        .anchor(anchor_block_header.try_into().unwrap())
        .with_component(get_item_component.clone())
        .with_component(auth_component)
        .storage_mode(storage_mode)
        .build()
        .unwrap();

    let proc_root = get_item_component.mast_forest().procedure_digests().next().unwrap();
    (account, seed, proc_root, secret_key)
}

/// Deploys a foreign account to the network with the specified code and storage mode. The account
/// is also inserted into the client and keystore.
///
/// # Returns
///
/// A tuple containing:
/// - `Account` - The deployed foreign account.
/// - `Digest` - The procedure root of the foreign account.
async fn deploy_foreign_account(
    client: &mut TestClient,
    keystore: &mut TestClientKeyStore,
    storage_mode: AccountStorageMode,
    code: String,
) -> Result<(Account, Digest), String> {
    let anchor_block = client.get_latest_epoch_block().await.unwrap();
    let (foreign_account, foreign_seed, proc_root, secret_key) =
        foreign_account_with_code(storage_mode, &anchor_block, code);
    let foreign_account_id = foreign_account.id();

    keystore.add_key(&AuthSecretKey::RpoFalcon512(secret_key)).unwrap();
    client.add_account(&foreign_account, Some(foreign_seed), false).await.unwrap();

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
                .build()
                .unwrap(),
        )
        .await
        .unwrap();
    let tx_id = tx.executed_transaction().id();
    client.submit_transaction(tx).await.unwrap();
    wait_for_tx(client, tx_id).await;

    Ok((foreign_account, proc_root))
}
