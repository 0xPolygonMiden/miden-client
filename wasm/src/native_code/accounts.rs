// use miden_lib::AuthScheme;
// use miden_objects::{
//     accounts::{Account, AccountType},
//     assets::TokenSymbol,
//     crypto::dsa::rpo_falcon512::KeyPair,
//     Felt, Word,
// };

use rand::{rngs::StdRng, Rng, SeedableRng};

use super::{
    rpc::NodeRpcClient, 
    Client, 
    store::Store // TODO: Add AuthInfo
};

pub enum AccountTemplate {
    BasicWallet {
        mutable_code: bool,
        storage_mode: AccountStorageMode,
    },
    FungibleFaucet {
        //token_symbol: TokenSymbol,
        decimals: u8,
        max_supply: u64,
        storage_mode: AccountStorageMode,
    },
}

pub enum AccountStorageMode {
    Local,
    OnChain,
}

impl<N: NodeRpcClient, S: Store> Client<N, S> {
    // ACCOUNT CREATION
    // --------------------------------------------------------------------------------------------

    /// Creates a new [Account] based on an [AccountTemplate] and saves it in the store
    pub async fn new_account(
        &mut self,
        template: AccountTemplate,
    ) -> String { // TODO: Replace with Result<(Account, Word), ()>
        let mut rng = StdRng::from_entropy();

        match template {
            AccountTemplate::BasicWallet {
                mutable_code,
                storage_mode,
            } => self.new_basic_wallet(mutable_code, &mut rng, storage_mode).await,
            AccountTemplate::FungibleFaucet {
                //token_symbol,
                decimals,
                max_supply,
                storage_mode,
            } => {
                self.new_fungible_faucet(decimals, max_supply, &mut rng, storage_mode).await
            }
        }
    }

    /// Creates a new regular account and saves it in the store along with its seed and auth data
    ///
    /// # Panics
    ///
    /// If the passed [AccountStorageMode] is [AccountStorageMode::OnChain], this function panics
    /// since this feature is not currently supported on Miden
    async fn new_basic_wallet(
        &mut self,
        mutable_code: bool,
        rng: &mut StdRng,
        account_storage_mode: AccountStorageMode,
    ) -> String { // TODO: Replace with Result<(Account, Word), ()>

        // if let AccountStorageMode::OnChain = account_storage_mode {
        //     todo!("Recording the account on chain is not supported yet");
        // }

        // let key_pair: KeyPair = KeyPair::new()?;

        // let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
        //     pub_key: key_pair.public_key(),
        // };

        // // we need to use an initial seed to create the wallet account
        // let init_seed: [u8; 32] = rng.gen();

        // let (account, seed) = if !mutable_code {
        //     miden_lib::accounts::wallets::create_basic_wallet(
        //         init_seed,
        //         auth_scheme,
        //         AccountType::RegularAccountImmutableCode,
        //     )
        // } else {
        //     miden_lib::accounts::wallets::create_basic_wallet(
        //         init_seed,
        //         auth_scheme,
        //         AccountType::RegularAccountUpdatableCode,
        //     )
        // }?;

        // self.insert_account(&account, Some(seed), &AuthInfo::RpoFalcon512(key_pair))?;
        // Ok((account, seed))

        "Called new_basic_wallet".to_string()
    }

    async fn new_fungible_faucet(
        &mut self,
        //token_symbol: TokenSymbol,
        decimals: u8,
        max_supply: u64,
        rng: &mut StdRng,
        account_storage_mode: AccountStorageMode,
    ) -> String{ // TODO: Replace with Result<(Account, Word), ()>

        // if let AccountStorageMode::OnChain = account_storage_mode {
        //     todo!("On-chain accounts are not supported yet");
        // }

        // let key_pair: KeyPair = KeyPair::new()?;

        // let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
        //     pub_key: key_pair.public_key(),
        // };

        // // we need to use an initial seed to create the wallet account
        // let init_seed: [u8; 32] = rng.gen();

        // let (account, seed) = miden_lib::accounts::faucets::create_basic_fungible_faucet(
        //     init_seed,
        //     token_symbol,
        //     decimals,
        //     Felt::try_from(max_supply.to_le_bytes().as_slice())
        //         .expect("u64 can be safely converted to a field element"),
        //     auth_scheme,
        // )?;

        // self.insert_account(&account, Some(seed), &AuthInfo::RpoFalcon512(key_pair))?;
        // Ok((account, seed))

        "Called new_fungible_faucet".to_string()
    }

    pub async fn get_account(
        &mut self,
        account_id: String // TODO: Replace with AccountId
    ) -> String { // TODO: Replace with Result<(Account, Option<Word>), ()>
        // self.store.get_account(account_id).map_err(|err| err.into())

        "Called get_account".to_string()
    }

    pub async fn get_accounts(
        &mut self
    ) -> String {  // TODO: Replace with Result<Vec<(AccountStub, Option<Word>)>, ()>
        // self.store.get_account_stubs().map_err(|err| err.into())
        
        "Called get_accounts".to_string()
    }

    pub async fn get_account_stub_by_id(
        &self,
        account_id: String, // TODO: Replace with AccountId
    ) -> String { // TODO: Replace with Result<(AccountStub, Option<Word>), ()>
        //self.store.get_account_stub(account_id).map_err(|err| err.into())

        "Called get_account_stub_by_id".to_string()
    }

    pub async fn get_account_auth(
        &mut self,
        account_id: String // TODO: Replace with AccountId
    ) -> String { // TODO: Replace with Result<AuthInfo, ()>
        // self.store.get_account_auth(account_id).map_err(|err| err.into())
        
        "Called get_account_auth".to_string()
    }

    // TODO: Import Account
}
