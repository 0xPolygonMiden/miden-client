use super::Client;
use crypto::utils::Deserializable;
use miden_node_store::genesis::GenesisState;
use std::{fs, path::Path};

#[cfg(test)]
use crate::errors::ClientError;
#[cfg(test)]
use objects::BlockHeader;

impl Client {
    #[cfg(test)]
    pub fn get_block_headers(
        &self,
        start: u32,
        finish: u32,
    ) -> Result<Vec<BlockHeader>, ClientError> {
        let mut headers = Vec::new();
        for block_number in start..=finish {
            if let Ok((block_header, _)) = self.store.get_block_header_by_num(block_number) {
                headers.push(block_header)
            }
        }

        Ok(headers)
    }

    pub fn load_genesis_data(
        &mut self,
        path: &Path,
        account_indices: Option<Vec<usize>>,
    ) -> Result<(), String> {
        let file_contents = fs::read(path.join("genesis.dat")).map_err(|err| err.to_string())?;

        let genesis_state =
            GenesisState::read_from_bytes(&file_contents).map_err(|err| err.to_string())?;

        let account_indices =
            account_indices.unwrap_or((0..genesis_state.accounts.len()).collect());

        for account_index in account_indices {
            let account_data_filepath = format!("accounts/account{}.mac", account_index);
            self.import_account_from_file(path.join(account_data_filepath))?;
        }
        Ok(())
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub mod tests {
    use crate::store::tests::create_test_store_path;
    use crypto::{utils::Serializable, Felt, FieldElement};
    use miden_client::{
        client::Client,
        config::{ClientConfig, Endpoint},
    };
    use miden_lib::transaction::TransactionKernel;
    use miden_node_store::genesis::GenesisState;
    use mock::{
        constants::{generate_account_seed, AccountSeedType},
        mock::account,
    };
    use objects::accounts::{Account, AccountData, AuthData};
    use rand::{rngs::ThreadRng, thread_rng, Rng};
    use std::{env::temp_dir, fs, path::PathBuf};

    fn create_account_data(
        rng: &mut ThreadRng,
        seed_type: AccountSeedType,
        account_file_path: PathBuf,
    ) -> Account {
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

        let account_data = AccountData::new(account.clone(), Some(account_seed), auth_data);
        fs::write(account_file_path, account_data.to_bytes()).unwrap();

        account
    }

    pub fn create_genesis_data() -> (PathBuf, Vec<Account>) {
        let temp_dir = temp_dir();
        let mut rng = thread_rng();

        let account_dir = temp_dir.join("accounts");
        fs::create_dir_all(account_dir.clone()).unwrap();

        let account = create_account_data(
            &mut rng,
            AccountSeedType::RegularAccountUpdatableCodeOnChain,
            account_dir.join("account0.mac"),
        );

        // Create a Faucet and save it to a file
        let faucet_account = create_account_data(
            &mut rng,
            AccountSeedType::FungibleFaucetValidInitialBalance,
            account_dir.join("account1.mac"),
        );

        // Create Genesis state and save it to a file
        let accounts = vec![account, faucet_account];
        let genesis_state = GenesisState::new(accounts.clone(), 1, 1);
        fs::write(temp_dir.join("genesis.dat"), genesis_state.to_bytes()).unwrap();

        (temp_dir, accounts)
    }

    #[tokio::test]
    async fn load_genesis_test() {
        // generate test store path
        let store_path = create_test_store_path();

        // generate test client
        let mut client = Client::new(ClientConfig::new(
            store_path.into_os_string().into_string().unwrap(),
            Endpoint::default(),
        ))
        .await
        .unwrap();

        let (genesis_data_path, created_accounts) = create_genesis_data();
        client.load_genesis_data(&genesis_data_path, None).unwrap();

        let accounts = client.get_accounts().unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].0.id(), created_accounts[0].id());
        assert_eq!(accounts[0].0.nonce(), created_accounts[0].nonce());
        assert_eq!(
            accounts[0].0.vault_root(),
            created_accounts[0].vault().commitment()
        );
        assert_eq!(
            accounts[0].0.storage_root(),
            created_accounts[0].storage().root()
        );
        assert_eq!(accounts[0].0.code_root(), created_accounts[0].code().root());

        assert_eq!(accounts[1].0.id(), created_accounts[1].id());
        assert_eq!(accounts[1].0.nonce(), created_accounts[1].nonce());
        assert_eq!(
            accounts[1].0.vault_root(),
            created_accounts[1].vault().commitment()
        );
        assert_eq!(
            accounts[1].0.storage_root(),
            created_accounts[1].storage().root()
        );
        assert_eq!(accounts[1].0.code_root(), created_accounts[1].code().root());
    }
}
