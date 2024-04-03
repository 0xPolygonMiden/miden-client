impl WebStore {
    pub(super) async fn get_account_ids(
        &mut self
    ) -> Result<Vec<AccountId>, ()> {
        let results: Vec<String> = idxdb_get_account_ids().await?;
  
        let converted_results: Result<Vec<AccountId>, ()> = results.into_iter().map(|id| {
            AccountId::from_hex(&id).map_err(|_err| ()) // Convert any error to `()`
        }).collect(); // Collect into a Result<Vec<AccountId>, ()>
        
        // Now, `converted_results` is of type Result<Vec<AccountId>, ()>, which you can return directly
        return converted_results;
    }

    pub(super) async fn get_account_stubs(
        &mut self
    ) ->  Result<Vec<(AccountStub, Option<Word>)>, ()> {
        let results = idxdb_get_account_stubs();

        let account_stubs_idxdb: Vec<AccountRecordIdxdbOjbect> = from_value(results).unwrap();
        
        // Convert each AccountRecordIdxdbObject to AccountStub (and pair with Option<Word>)
        let account_stubs: Vec<(AccountStub, Option<Word>)> = account_stubs_idxdb.into_iter().map(|record| {
            // Need to convert the hex string back to AccountId to then turn it into a u64
            let native_account_id: i64 = AccountId::from_hex(&record.id).map_err(|err| err.to_string())?;
            let native_nonce: i64 = record.nonce.parse().unwrap();
            let account_seed = record.account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose()?;
            
            // Convert AccountRecordIdxdbObject to AccountStub here
            let account_stub = AccountStub {
                id: native_account_id,
                nonce: native_nonce,
                vault_root: record.vault_root,
                storage_root: record.storage_root,
                code_root: record.code_root,
                account_seed: account_seed,
            };

            // Pair AccountStub with Option<Word>, assuming we don't have a Word value to include
            (account_stub, None) // Adjust this as needed based on how you derive Word from your data
        }).collect();

        Ok(account_stubs)
    }

    pub(crate) async fn get_account_stub(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Option<Word>), ()> {
        // Need to convert AccountId to hex string to match the IndexedDB key
        let account_id_str = AccountId::to_hex(account_id).map_err(|err| err.to_string())?;
        let result = idxdb_get_account_stub(account_id_str);
        let account_stub_idxdb: AccountRecordIdxdbOjbect = from_value(result).unwrap();

        // Need to convert the hex string back to AccountId to then turn it into a u64
        let native_account_id: i64 = AccountId::from_hex(&account_stub_idxdb.id).map_err(|err| err.to_string())?;
        let native_nonce: i64 = account_stub_idxdb.nonce.parse().unwrap();
        let account_seed = account_stub_idxdb.account_seed.map(|seed| Word::read_from_bytes(&seed)).transpose()?;

        Ok((
            AccountStub::new(
                (native_account_id as u64)
                    .try_into()
                    .expect("Conversion from stored AccountID should not panic"),
                Felt::new(native_nonce as u64),
                serde_json::from_str(&vault_root).map_err(StoreError::JsonDataDeserializationError)?,
                Digest::try_from(&storage_root)?,
                Digest::try_from(&code_root)?,
            ),
            account_seed,
        ));
    }

    pub(crate) async fn get_account(
        &mut self,
        account_id: AccountId
    ) -> Result<(), ()> { // TODO: Replace with  Result<(Account, Option<Word>), ()>
        let (account_stub, seed) = self.get_account_stub(account_id)?;
        let (_procedures, module_ast) = self.get_account_code(account_stub.code_root())?;

        let account_code = AccountCode::new(module_ast, &TransactionKernel::assembler()).unwrap();

        let account_storage = self.get_account_storage(account_stub.storage_root())?;

        let account_vault = self.get_vault_assets(account_stub.vault_root())?;
        let account_vault = AssetVault::new(&account_vault)?;

        let account = Account::new(
            account_stub.id(),
            account_vault,
            account_storage,
            account_code,
            account_stub.nonce(),
        );

        Ok((account, seed))
    }

    pub(crate) async fn get_account_auth(
        &mut self,
        account_id: AccountId
    ) -> Result<AuthInfo, ()> {
        let account_id_str = AccountId::to_hex(account_id).map_err(|err| err.to_string())?;
        let result = idxdb_get_account_auth(account_id_str);
        let auth_info_idxdb: AccountAuthIdxdbObject = from_value(result).unwrap();
        
        // Convert the auth_info to the appropriate AuthInfo enum variant
        let auth_info = AuthInfo::from_bytes(&auth_info_idxdb.auth_info);
        Ok(auth_info)
    }

    pub(super) async fn get_account_code(
        &mut self,
        root: Digest
    ) -> Result<(Vec<Digest>, ModuleAst), ()> {
        let root_serialized = root.to_string();
        let result = idxdb_get_account_code(root_serialized);
        let account_code_idxdb: AccountCodeIdxdbObject = from_value(result).unwrap();

        let procedures =
            serde_json::from_str(&account_code_idxdb.procedures)?;
        let module = ModuleAst::from_bytes(&account_code_idxdb.module)?;
        Ok((procedures, module));
    }
    pub(super) async fn get_account_storage(
        &mut self,
        root: Digest
    ) -> Result<AccountStorage, ()> {
        let root_serialized = &root.to_string();

        let result = idxdb_get_account_storage(root_serialized);
        let account_code_idxdb: AccountStorageIdxdbObject = from_value(result).unwrap();

        let storage = AccountStorage::from_bytes(&account_code_idxdb.storage);
        Ok(storage)
    }

    pub(super) async fn get_vault_assets(
        &mut self,
        root: Digest
    ) -> Result<Vec<Asset>, ()> {
        let root_serialized = &root.to_string();

        let result = idxdb_get_vault_assets(root_serialized);
        let vault_assets_idxdb: AccountVaultIdxdbObject = from_value(result).unwrap();

        let assets = serde_json::from_str(&assets);
        Ok(assets)
    }

    pub(crate) async fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Option<Word>,
        auth_info: &AuthInfo,
    ) -> Result<(), ()> {
        insert_account_code(account.code()).await?;
        insert_account_storage(account.storage()).await?;
        insert_account_asset_vault(account.vault()).await?;
        insert_account_record(account, account_seed).await?;
        insert_account_auth(account.id(), auth_info).await?;

        Ok(())
    }
}