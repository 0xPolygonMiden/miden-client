use miden_client::{
    accounts::{Account, AccountData, AccountTemplate, StorageSlot},
    testing::prepare_word,
    transactions::{TransactionKernel, TransactionRequest},
    Felt, Word,
};
use miden_lib::accounts::auth::RpoFalcon512;
use miden_objects::{
    accounts::{AccountBuilder, AccountComponent, AccountStorageMode, AuthSecretKey},
    crypto::dsa::rpo_falcon512::SecretKey,
    transaction::TransactionScript,
    Digest,
};

use super::common::*;

// FPI TESTS
// ================================================================================================

const FPI_STORAGE_VALUE: Word =
    [Felt::new(9u64), Felt::new(12u64), Felt::new(18u64), Felt::new(30u64)];

#[tokio::test]
async fn test_fpi() {
    let mut client = create_test_client().await;
    wait_for_node(&mut client).await;

    let (foreign_account, foreign_seed, secret_key, proc_root) = foreign_account();

    let foreign_account_id = foreign_account.id();

    client
        .import_account(AccountData::new(
            foreign_account,
            Some(foreign_seed),
            AuthSecretKey::RpoFalcon512(secret_key.clone()),
        ))
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
            TransactionRequest::new().with_custom_script(deployment_tx_script).unwrap(),
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

    let tx_result = client
        .new_transaction(
            native_account.id(),
            TransactionRequest::new()
                .with_public_foreign_accounts([foreign_account_id])
                .unwrap()
                .with_custom_script(tx_script)
                .unwrap(),
        )
        .await
        .unwrap();

    client.submit_transaction(tx_result).await.unwrap();
}

/// Builds an account using the auth component and a custom component which just retrieves the
/// value stored in its first slot.
/// This function also returns the seed, the secret key and the procedure root for this custom
/// component's procedure, used for FPI on a separate account.
pub fn foreign_account() -> (Account, Word, SecretKey, Digest) {
    let get_item_component = AccountComponent::compile(
        "
        export.get_fpi_item
            push.0
            exec.::miden::account::get_item
            swapw dropw
        end
        ",
        TransactionKernel::assembler(),
        vec![StorageSlot::Value(FPI_STORAGE_VALUE)],
    )
    .unwrap()
    .with_supports_all_types();

    let secret_key = SecretKey::new();
    let auth_component = RpoFalcon512::new(secret_key.public_key());

    let (account, seed) = AccountBuilder::new()
        .init_seed(Default::default())
        .with_component(get_item_component.clone())
        .with_component(auth_component)
        .storage_mode(AccountStorageMode::Public)
        .build()
        .unwrap();

    let proc_root = get_item_component.mast_forest().procedure_digests().next().unwrap();
    (account, seed, secret_key, proc_root)
}
