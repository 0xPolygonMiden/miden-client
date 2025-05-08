use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    AccountIdError, Digest, Word,
    account::{Account, AccountCode, AccountHeader, AccountId, AccountStorage},
    asset::{Asset, AssetVault},
};
use miden_tx::utils::{Deserializable, Serializable};
use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::JsFuture;

use super::WebStore;
use crate::store::{AccountRecord, AccountStatus, StoreError};

mod js_bindings;
use js_bindings::{
    idxdb_fetch_and_cache_account_auth_by_pub_key, idxdb_get_account_asset_vault,
    idxdb_get_account_code, idxdb_get_account_header, idxdb_get_account_header_by_commitment,
    idxdb_get_account_headers, idxdb_get_account_ids, idxdb_get_account_storage,
    idxdb_get_foreign_account_code, idxdb_lock_account, idxdb_undo_account_states,
    idxdb_upsert_foreign_account_code,
};

mod models;
use models::{
    AccountAuthIdxdbObject, AccountCodeIdxdbObject, AccountRecordIdxdbObject,
    AccountStorageIdxdbObject, AccountVaultIdxdbObject, ForeignAcountCodeIdxdbObject,
};

pub(crate) mod utils;
use utils::{
    insert_account_asset_vault, insert_account_code, insert_account_record, insert_account_storage,
    parse_account_record_idxdb_object, update_account,
};

impl WebStore {
    pub(super) async fn get_account_ids(&self) -> Result<Vec<AccountId>, StoreError> {
        let promise = idxdb_get_account_ids();
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to fetch account ids: {js_error:?}",))
        })?;
        let account_ids_as_strings: Vec<String> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        let native_account_ids: Vec<AccountId> = account_ids_as_strings
            .into_iter()
            .map(|id| AccountId::from_hex(&id))
            .collect::<Result<Vec<_>, AccountIdError>>()?;

        Ok(native_account_ids)
    }

    pub(super) async fn get_account_headers(
        &self,
    ) -> Result<Vec<(AccountHeader, AccountStatus)>, StoreError> {
        let promise = idxdb_get_account_headers();
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to fetch account headers: {js_error:?}",))
        })?;

        let account_headers_idxdb: Vec<AccountRecordIdxdbObject> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        let account_headers: Vec<(AccountHeader, AccountStatus)> = account_headers_idxdb
            .into_iter()
            .map(parse_account_record_idxdb_object)
            .collect::<Result<Vec<_>, StoreError>>()?;

        Ok(account_headers)
    }

    pub(crate) async fn get_account_header(
        &self,
        account_id: AccountId,
    ) -> Result<Option<(AccountHeader, AccountStatus)>, StoreError> {
        let account_id_str = account_id.to_string();
        let promise = idxdb_get_account_header(account_id_str);

        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to fetch account header: {js_error:?}",))
        })?;

        let account_header_idxdb: Option<AccountRecordIdxdbObject> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        match account_header_idxdb {
            None => Ok(None),
            Some(account_header_idxdb) => {
                let parsed_account_record =
                    parse_account_record_idxdb_object(account_header_idxdb)?;

                Ok(Some(parsed_account_record))
            },
        }
    }

    pub(crate) async fn get_account_header_by_commitment(
        &self,
        account_commitment: Digest,
    ) -> Result<Option<AccountHeader>, StoreError> {
        let account_commitment_str = account_commitment.to_string();

        let promise = idxdb_get_account_header_by_commitment(account_commitment_str);
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!(
                "failed to fetch account header by commitment: {js_error:?}",
            ))
        })?;

        let account_header_idxdb: Option<AccountRecordIdxdbObject> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        let account_header: Result<Option<AccountHeader>, StoreError> = account_header_idxdb
            .map_or(Ok(None), |account_record| {
                let result = parse_account_record_idxdb_object(account_record);

                result.map(|(account_header, _status)| Some(account_header))
            });

        account_header
    }

    pub(crate) async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountRecord>, StoreError> {
        let (account_header, status) = match self.get_account_header(account_id).await? {
            None => return Ok(None),
            Some((account_header, status)) => (account_header, status),
        };
        let account_code = self.get_account_code(account_header.code_commitment()).await?;

        let account_storage = self.get_account_storage(account_header.storage_commitment()).await?;
        let account_vault = self.get_vault_assets(account_header.vault_root()).await?;
        let account_vault = AssetVault::new(&account_vault)?;

        let account = Account::from_parts(
            account_header.id(),
            account_vault,
            account_storage,
            account_code,
            account_header.nonce(),
        );

        Ok(Some(AccountRecord::new(account, status)))
    }

    pub(super) async fn get_account_code(&self, root: Digest) -> Result<AccountCode, StoreError> {
        let root_serialized = root.to_string();

        let promise = idxdb_get_account_code(root_serialized);
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to fetch account code: {js_error:?}",))
        })?;

        let account_code_idxdb: AccountCodeIdxdbObject = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        let code =
            AccountCode::from_bytes(&account_code_idxdb.code).map_err(StoreError::AccountError)?;

        Ok(code)
    }

    pub(super) async fn get_account_storage(
        &self,
        commitment: Digest,
    ) -> Result<AccountStorage, StoreError> {
        let commitment_serialized = commitment.to_string();

        let promise = idxdb_get_account_storage(commitment_serialized);
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to fetch account storage: {js_error:?}",))
        })?;

        let account_storage_idxdb: AccountStorageIdxdbObject = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        Ok(AccountStorage::read_from_bytes(&account_storage_idxdb.storage)?)
    }

    pub(super) async fn get_vault_assets(
        &self,
        commitment: Digest,
    ) -> Result<Vec<Asset>, StoreError> {
        let commitment_serialized = commitment.to_string();

        let promise = idxdb_get_account_asset_vault(commitment_serialized);
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to fetch vault assets: {js_error:?}",))
        })?;
        let vault_assets_idxdb: AccountVaultIdxdbObject = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        Ok(Vec::<Asset>::read_from_bytes(&vault_assets_idxdb.assets)?)
    }

    pub(crate) async fn insert_account(
        &self,
        account: &Account,
        account_seed: Option<Word>,
    ) -> Result<(), StoreError> {
        insert_account_code(account.code()).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to insert account code: {js_error:?}",))
        })?;

        insert_account_storage(account.storage()).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to insert account storage:{js_error:?}",))
        })?;

        insert_account_asset_vault(account.vault()).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to insert account vault:{js_error:?}",))
        })?;

        insert_account_record(account, account_seed).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to insert account record: {js_error:?}",))
        })?;

        Ok(())
    }

    pub(crate) async fn update_account(
        &self,
        new_account_state: &Account,
    ) -> Result<(), StoreError> {
        let account_id_str = new_account_state.id().to_string();
        let promise = idxdb_get_account_header(account_id_str);

        if JsFuture::from(promise).await.is_err() {
            return Err(StoreError::AccountDataNotFound(new_account_state.id()));
        }

        update_account(new_account_state)
            .await
            .map_err(|_| StoreError::DatabaseError("failed to update account".to_string()))
    }

    pub async fn fetch_and_cache_account_auth_by_pub_key(
        &self,
        pub_key: String,
    ) -> Result<Option<String>, StoreError> {
        let promise = idxdb_fetch_and_cache_account_auth_by_pub_key(pub_key);

        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!(
                "failed to fetch and cache account auth by pub key: {js_error:?}",
            ))
        })?;

        let account_auth_idxdb: Option<AccountAuthIdxdbObject> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        match account_auth_idxdb {
            None => Ok(None),
            Some(account_auth_idxdb) => {
                // Convert the auth_info to the appropriate AuthInfo enum variant
                Ok(Some(account_auth_idxdb.secret_key))
            },
        }
    }

    pub(crate) async fn upsert_foreign_account_code(
        &self,
        account_id: AccountId,
        code: AccountCode,
    ) -> Result<(), StoreError> {
        let root = code.commitment().to_string();
        let code = code.to_bytes();
        let account_id = account_id.to_string();

        let promise = idxdb_upsert_foreign_account_code(account_id, code, root);
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!(
                "failed to upsert foreign account code: {js_error:?}",
            ))
        })?;

        Ok(())
    }

    pub(crate) async fn get_foreign_account_code(
        &self,
        account_ids: Vec<AccountId>,
    ) -> Result<BTreeMap<AccountId, AccountCode>, StoreError> {
        let account_ids = account_ids.iter().map(ToString::to_string).collect::<Vec<_>>();
        let promise = idxdb_get_foreign_account_code(account_ids);
        let js_value = JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(
                format!("failed to fetch foreign account code: {js_error:?}",),
            )
        })?;

        let foreign_account_code_idxdb: Vec<ForeignAcountCodeIdxdbObject> = from_value(js_value)
            .map_err(|err| StoreError::DatabaseError(format!("failed to deserialize {err:?}")))?;

        let foreign_account_code: BTreeMap<AccountId, AccountCode> = foreign_account_code_idxdb
            .into_iter()
            .map(|idxdb_object| {
                let account_id = AccountId::from_hex(&idxdb_object.account_id)
                    .map_err(StoreError::AccountIdError)?;
                let code = AccountCode::from_bytes(&idxdb_object.code)
                    .map_err(StoreError::AccountError)?;

                Ok((account_id, code))
            })
            .collect::<Result<BTreeMap<AccountId, AccountCode>, StoreError>>()?;

        Ok(foreign_account_code)
    }

    pub(crate) async fn undo_account_states(
        &self,
        account_states: &[Digest],
    ) -> Result<(), StoreError> {
        let account_commitments =
            account_states.iter().map(ToString::to_string).collect::<Vec<_>>();
        let promise = idxdb_undo_account_states(account_commitments);
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to undo account states: {js_error:?}",))
        })?;

        Ok(())
    }

    /// Locks the account if the mismatched digest doesn't belong to a previous account state (stale
    /// data).
    pub(crate) async fn lock_account_on_unexpected_commitment(
        &self,
        account_id: &AccountId,
        mismatched_digest: &Digest,
    ) -> Result<(), StoreError> {
        // Mismatched digests may be due to stale network data. If the mismatched digest is
        // tracked in the db and corresponds to the mismatched account, it means we
        // got a past update and shouldn't lock the account.
        if let Some(account) = self.get_account_header_by_commitment(*mismatched_digest).await? {
            if account.id() == *account_id {
                return Ok(());
            }
        }

        let account_id_str = account_id.to_string();
        let promise = idxdb_lock_account(account_id_str);
        JsFuture::from(promise).await.map_err(|js_error| {
            StoreError::DatabaseError(format!("failed to lock account: {js_error:?}",))
        })?;

        Ok(())
    }
}
