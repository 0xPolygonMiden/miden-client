use crypto::{dsa::rpo_falcon512::KeyPair, Felt, Word};
use miden_lib::AuthScheme;
use objects::{
    accounts::{
        Account, AccountData, AccountDelta, AccountId, AccountStorage, AccountStub, AccountType,
        AuthData,
    },
    assembly::ModuleAst,
    assets::{Asset, TokenSymbol},
    Digest,
};
use rand::{rngs::ThreadRng, Rng};

use crate::{errors::ClientError, store::accounts::AuthInfo};

use super::Client;

pub enum AccountTemplate {
    BasicWallet {
        mutable_code: bool,
        storage_mode: AccountStorageMode,
    },
    FungibleFaucet {
        token_symbol: TokenSymbol,
        decimals: u8,
        max_supply: u64,
        storage_mode: AccountStorageMode,
    },
}

pub enum AccountStorageMode {
    Local,
    OnChain,
}

impl Client {
    // ACCOUNT CREATION
    // --------------------------------------------------------------------------------------------

    /// Creates a new [Account] based on an [AccountTemplate] and saves it in the store
    pub fn new_account(
        &mut self,
        template: AccountTemplate,
    ) -> Result<(Account, Word), ClientError> {
        let mut rng = rand::thread_rng();

        let account_and_seed = match template {
            AccountTemplate::BasicWallet {
                mutable_code,
                storage_mode,
            } => self.new_basic_wallet(mutable_code, &mut rng, storage_mode),
            AccountTemplate::FungibleFaucet {
                token_symbol,
                decimals,
                max_supply,
                storage_mode,
            } => {
                self.new_fungible_faucet(token_symbol, decimals, max_supply, &mut rng, storage_mode)
            }
        }?;

        Ok(account_and_seed)
    }

    /// Saves in the store the [Account] corresponding to `account_data`. It is expected that the
    /// provided [AccountData] has an account_seed, otherwise panics.
    pub fn import_account(&mut self, account_data: AccountData) -> Result<(), ClientError> {
        match account_data.auth {
            AuthData::RpoFalcon512Seed(key_pair) => {
                let keypair = KeyPair::from_seed(&key_pair)?;
                let is_new_account = account_data.account.is_new();
                match account_data.account_seed {
                    Some(seed) if is_new_account => self.insert_account(
                        &account_data.account,
                        seed,
                        &AuthInfo::RpoFalcon512(keypair),
                    ),
                    Some(_) => {
                        tracing::warn!(
                            "Imported an existing account and still provided a seed when it is not needed. It's possible that the account's file was incorrectly generated."
                        );

                        unimplemented!();
                    }
                    None if !is_new_account => {
                        unimplemented!();
                    }
                    None => Err(ClientError::ImportAccountError(
                        "tried to import a new account without its seed".to_string(),
                    )),
                }
            }
        }
    }

    /// Creates a new regular account and saves it in the store along with its seed and auth data
    fn new_basic_wallet(
        &mut self,
        mutable_code: bool,
        rng: &mut ThreadRng,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError> {
        if let AccountStorageMode::OnChain = account_storage_mode {
            todo!("Recording the account on chain is not supported yet");
        }

        let key_pair: objects::crypto::dsa::rpo_falcon512::KeyPair =
            objects::crypto::dsa::rpo_falcon512::KeyPair::new()?;

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
            pub_key: key_pair.public_key(),
        };

        // we need to use an initial seed to create the wallet account
        let init_seed: [u8; 32] = rng.gen();

        let (account, seed) = if !mutable_code {
            miden_lib::accounts::wallets::create_basic_wallet(
                init_seed,
                auth_scheme,
                AccountType::RegularAccountImmutableCode,
            )
        } else {
            miden_lib::accounts::wallets::create_basic_wallet(
                init_seed,
                auth_scheme,
                AccountType::RegularAccountUpdatableCode,
            )
        }?;

        self.insert_account(&account, seed, &AuthInfo::RpoFalcon512(key_pair))?;
        Ok((account, seed))
    }

    fn new_fungible_faucet(
        &mut self,
        token_symbol: TokenSymbol,
        decimals: u8,
        max_supply: u64,
        rng: &mut ThreadRng,
        account_storage_mode: AccountStorageMode,
    ) -> Result<(Account, Word), ClientError> {
        if let AccountStorageMode::OnChain = account_storage_mode {
            todo!("On-chain accounts are not supported yet");
        }

        let key_pair: objects::crypto::dsa::rpo_falcon512::KeyPair =
            objects::crypto::dsa::rpo_falcon512::KeyPair::new()?;

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
            pub_key: key_pair.public_key(),
        };

        // we need to use an initial seed to create the wallet account
        let init_seed: [u8; 32] = rng.gen();

        let (account, seed) = miden_lib::accounts::faucets::create_basic_fungible_faucet(
            init_seed,
            token_symbol,
            decimals,
            Felt::try_from(max_supply.to_le_bytes().as_slice())
                .expect("u64 can be safely converted to a field element"),
            auth_scheme,
        )?;

        self.insert_account(&account, seed, &AuthInfo::RpoFalcon512(key_pair))?;
        Ok((account, seed))
    }

    /// Inserts a new account into the client's store.
    pub fn insert_account(
        &mut self,
        account: &Account,
        account_seed: Word,
        auth_info: &AuthInfo,
    ) -> Result<(), ClientError> {
        self.store
            .insert_account(account, account_seed, auth_info)
            .map_err(ClientError::StoreError)
    }

    /// Applies an [AccountDelta] to the stored account and stores the result in the database.
    pub fn update_account(
        &mut self,
        account_id: AccountId,
        account_delta: &AccountDelta,
    ) -> Result<(), ClientError> {
        self.store
            .update_account(account_id, account_delta)
            .map_err(ClientError::StoreError)
    }

    // ACCOUNT DATA RETRIEVAL
    // --------------------------------------------------------------------------------------------

    /// Returns summary info about the accounts managed by this client.
    ///
    pub fn get_accounts(&self) -> Result<Vec<(AccountStub, Word)>, ClientError> {
        self.store.get_accounts().map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    pub fn get_account_by_id(&self, account_id: AccountId) -> Result<(Account, Word), ClientError> {
        self.store
            .get_account_by_id(account_id)
            .map_err(|err| err.into())
    }

    /// Returns summary info about the specified account.
    pub fn get_account_stub_by_id(
        &self,
        account_id: AccountId,
    ) -> Result<(AccountStub, Word), ClientError> {
        self.store
            .get_account_stub_by_id(account_id)
            .map_err(|err| err.into())
    }

    /// Returns key pair structure for an Account Id.
    pub fn get_account_auth(&self, account_id: AccountId) -> Result<AuthInfo, ClientError> {
        self.store
            .get_account_auth(account_id)
            .map_err(|err| err.into())
    }

    /// Returns vault assets from a vault root.
    pub fn get_vault_assets(&self, vault_root: Digest) -> Result<Vec<Asset>, ClientError> {
        self.store
            .get_vault_assets(vault_root)
            .map_err(|err| err.into())
    }

    /// Returns account code data from a root.
    pub fn get_account_code(
        &self,
        code_root: Digest,
    ) -> Result<(Vec<Digest>, ModuleAst), ClientError> {
        self.store
            .get_account_code(code_root)
            .map_err(|err| err.into())
    }

    /// Returns account storage data from a storage root.
    pub fn get_account_storage(&self, storage_root: Digest) -> Result<AccountStorage, ClientError> {
        self.store
            .get_account_storage(storage_root)
            .map_err(|err| err.into())
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use crate::store::tests::create_test_client;
    use crypto::{Felt, FieldElement};

    use miden_lib::transaction::TransactionKernel;
    use mock::{
        constants::{generate_account_seed, AccountSeedType},
        mock::account,
    };
    use objects::accounts::{AccountData, AuthData};
    use rand::{rngs::ThreadRng, thread_rng, Rng};

    fn create_account_data(rng: &mut ThreadRng, seed_type: AccountSeedType) -> AccountData {
        // Create an account and save it to a file
        let (account_id, account_seed) = generate_account_seed(seed_type);
        let assembler = TransactionKernel::assembler();
        let account = account::mock_account(Some(account_id.into()), Felt::ZERO, None, &assembler);

        let key_pair_seed: [u32; 10] = rng.gen();
        let mut key_pair_seed_u8: [u8; 40] = [0; 40];
        for (dest_c, source_e) in key_pair_seed_u8
            .chunks_exact_mut(4)
            .zip(key_pair_seed.iter())
        {
            dest_c.copy_from_slice(&source_e.to_le_bytes())
        }
        let auth_data = AuthData::RpoFalcon512Seed(key_pair_seed_u8);

        AccountData::new(account.clone(), Some(account_seed), auth_data)
    }

    pub fn create_initial_accounts_data() -> Vec<AccountData> {
        let mut rng = thread_rng();
        let account = create_account_data(
            &mut rng,
            AccountSeedType::RegularAccountUpdatableCodeOnChain,
        );

        // Create a Faucet and save it to a file
        let faucet_account =
            create_account_data(&mut rng, AccountSeedType::FungibleFaucetValidInitialBalance);

        // Create Genesis state and save it to a file
        let accounts = vec![account, faucet_account];

        accounts
    }

    #[tokio::test]
    async fn load_accounts_test() {
        // generate test client
        let mut client = create_test_client();

        let created_accounts_data = create_initial_accounts_data();

        for account_data in created_accounts_data.clone() {
            client.import_account(account_data).unwrap();
        }

        let expected_accounts: Vec<_> = created_accounts_data
            .into_iter()
            .map(|account_data| account_data.account)
            .collect();
        let accounts = client.get_accounts().unwrap();

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].0.id(), expected_accounts[0].id());
        assert_eq!(accounts[0].0.nonce(), expected_accounts[0].nonce());
        assert_eq!(
            accounts[0].0.vault_root(),
            expected_accounts[0].vault().commitment()
        );
        assert_eq!(
            accounts[0].0.storage_root(),
            expected_accounts[0].storage().root()
        );
        assert_eq!(
            accounts[0].0.code_root(),
            expected_accounts[0].code().root()
        );

        assert_eq!(accounts[1].0.id(), expected_accounts[1].id());
        assert_eq!(accounts[1].0.nonce(), expected_accounts[1].nonce());
        assert_eq!(
            accounts[1].0.vault_root(),
            expected_accounts[1].vault().commitment()
        );
        assert_eq!(
            accounts[1].0.storage_root(),
            expected_accounts[1].storage().root()
        );
        assert_eq!(
            accounts[1].0.code_root(),
            expected_accounts[1].code().root()
        );
    }
}
