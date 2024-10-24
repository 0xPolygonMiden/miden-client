use miden_client::{
    accounts::{AccountCode, AccountData, AccountTemplate},
    testing::{account::AccountBuilder, prepare_word},
    transactions::{TransactionKernel, TransactionRequest},
};
use miden_objects::{
    accounts::{AccountStorageMode, AuthSecretKey},
    transaction::TransactionScript,
    Digest,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use super::common::*;

// FPI TESTS
// ================================================================================================

#[tokio::test]
async fn test_fpi() {
    let mut client = create_test_client();
    wait_for_node(&mut client).await;

    let (foreign_account, foreign_seed, secret_key) =
        AccountBuilder::new(ChaCha20Rng::from_entropy())
            .code(foreign_account_code())
            .storage_mode(AccountStorageMode::Public)
            .build_with_auth(&mut ChaCha20Rng::from_entropy())
            .unwrap();

    let foreign_account_id = foreign_account.id();

    client
        .import_account(AccountData::new(
            foreign_account,
            Some(foreign_seed),
            AuthSecretKey::RpoFalcon512(secret_key),
        ))
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
            storage_mode: AccountStorageMode::Private,
        })
        .unwrap();

    let tx_script = format!(
        "
    use.miden::tx
    use.miden::account
    begin
        # pad the stack for the `execute_foreign_procedure`execution
        padw padw push.0.0.0
        # => [pad(11)]

        # push the index of desired storage item
        push.0

        # push hash of foreign proc - push_number
        # this can be done with the procref call, but it needs to be in the assembler
        push.{push_number_root}

        # push the foreign account id
        push.{foreign_account_id}
        # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(11)]

        exec.tx::execute_foreign_procedure
        # => [9]

        eq.9 assert
    end
    ",
        push_number_root = prepare_word(
            &Digest::try_from("0x805f708a5cfdb803c0255859bf001e9e5ae35d50baf8531ff35b3cd852a5eb6e")
                .unwrap()
        ),
    );

    let tx_script =
        TransactionScript::compile(tx_script, vec![], TransactionKernel::assembler()).unwrap();
    let _ = client.sync_state().await;
    let _tx_result = client
        .new_transaction(
            native_account.id(),
            TransactionRequest::new()
                .with_foreign_public_accounts([foreign_account_id])
                .with_custom_script(tx_script)
                .unwrap(),
        )
        .await
        .unwrap();
}

pub fn foreign_account_code() -> AccountCode {
    let code = AccountCode::compile(
        "
    use.miden::account
        
    export.::miden::contracts::wallets::basic::receive_asset
    export.::miden::contracts::wallets::basic::create_note
    export.::miden::contracts::wallets::basic::move_asset_to_note
    export.::miden::contracts::auth::basic::auth_tx_rpo_falcon512

    export.push_number
        push.9
    end

    export.push_storage_value
        push.0
        exec.account::get_item
        movup.8 drop movup.8 drop movup.8 drop
    end
    ",
        TransactionKernel::assembler(),
        false,
    )
    .unwrap();

    // todo: remove
    for proc in code.procedure_roots() {
        println!("proc roots {}", proc);
    }

    code
}
