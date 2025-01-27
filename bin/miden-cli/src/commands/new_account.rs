use std::{collections::BTreeMap, fs, io::Write, path::PathBuf};

use clap::{Parser, ValueEnum};
use miden_client::{
    account::{
        component::{
            AccountComponent, AccountComponentTemplate, BasicFungibleFaucet, BasicWallet,
            InitStorageData, MapRepresentation, PlaceholderType, RpoFalcon512, StorageValue,
        },
        AccountBuilder, AccountStorageMode, AccountType,
    },
    assets::TokenSymbol,
    auth::AuthSecretKey,
    crypto::{FeltRng, SecretKey},
    utils::Deserializable,
    Client, Felt, Word,
};
use miden_lib::utils::parse_hex_string_as_word;

use crate::{
    commands::account::maybe_set_default_account, errors::CliError, utils::load_config_file,
    CLIENT_BINARY_NAME,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliAccountStorageMode {
    Private,
    Public,
}

impl From<CliAccountStorageMode> for AccountStorageMode {
    fn from(cli_mode: CliAccountStorageMode) -> Self {
        match cli_mode {
            CliAccountStorageMode::Private => AccountStorageMode::Private,
            CliAccountStorageMode::Public => AccountStorageMode::Public,
        }
    }
}

/// Helper function to process extra component templates.
/// It reads user input for each placeholder in a component template.
// TODO: this could take a TOML file with key-values
fn process_component_templates(
    extra_components: &[AccountComponentTemplate],
) -> Result<Vec<AccountComponent>, CliError> {
    let mut account_components = vec![];
    for component_template in extra_components {
        let mut init_storage_data = BTreeMap::new();
        for (placeholder_key, placeholder_type) in
            component_template.metadata().get_unique_storage_placeholders()
        {
            print!(
                "Enter hex value for placeholder '{}' (type: {}): ",
                placeholder_key, placeholder_type
            );
            std::io::stdout().flush()?;

            let mut input_value = String::new();
            std::io::stdin().read_line(&mut input_value)?;
            let input_value = input_value.trim();

            match placeholder_type {
                PlaceholderType::Felt => {
                    let value = input_value
                        .strip_prefix("0x")
                        .ok_or("error parsing input: Missing 0x prefix".to_string())
                        .map(|hex| {
                            u64::from_str_radix(hex, 16).map_err(|e| {
                                CliError::Parse(e.into(), "failed to parse hex from input".into())
                            })
                        })
                        .map_err(|e| {
                            CliError::Parse(e.into(), "failed to parse hex from input".into())
                        })??;

                    init_storage_data
                        .insert(placeholder_key.clone(), StorageValue::Felt(Felt::new(value)));
                },
                PlaceholderType::Map => {
                    // TODO: Test this case further
                    let map: MapRepresentation = toml::from_str(input_value).map_err(|e| {
                        CliError::Parse(e.into(), "failed to parse map from input".into())
                    })?;
                    let map = map.try_build_map(&Default::default()).map_err(|e| {
                        CliError::Parse(e.into(), "failed to parse map from input".into())
                    })?;

                    init_storage_data.insert(placeholder_key.clone(), StorageValue::Map(map));
                },
                PlaceholderType::Word => {
                    let word: Word = parse_hex_string_as_word(input_value).map_err(|e| {
                        CliError::Parse(e.into(), "failed to parse hex from input".into())
                    })?;

                    init_storage_data.insert(placeholder_key.clone(), StorageValue::Word(word));
                },
            }
        }
        let component = AccountComponent::from_template(
            component_template,
            &InitStorageData::new(init_storage_data),
        )
        .map_err(|e| CliError::Account(e, "error instantiating component from template".into()))?
        .with_supports_all_types();

        account_components.push(component);
    }

    Ok(account_components)
}

// NEW FAUCET
// ================================================================================================

/// Create a new faucet account.
#[derive(Debug, Parser, Clone)]
pub struct NewFaucetCmd {
    /// Storage mode of the account.
    #[clap(value_enum, short, long, default_value_t = CliAccountStorageMode::Private)]
    storage_mode: CliAccountStorageMode,
    /// Defines if the account assets are non-fungible (by default it is fungible).
    #[clap(short, long)]
    non_fungible: bool,
    /// Token symbol of the faucet.
    #[clap(short, long)]
    token_symbol: Option<String>,
    /// Decimals of the faucet.
    #[clap(short, long)]
    decimals: Option<u8>,
    /// Maximum amount of assets that the fungible faucet can distribute.
    #[clap(short, long)]
    max_supply: Option<u64>,
    /// Optional list of files specifying addional components to add to the account.
    #[clap(short, long)]
    extra_components: Vec<PathBuf>,
}

impl NewFaucetCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), CliError> {
        if self.non_fungible {
            todo!("Non-fungible faucets are not supported yet");
        }

        if self.token_symbol.is_none() || self.decimals.is_none() || self.max_supply.is_none() {
            return Err(CliError::MissingFlag(
                "`token-symbol`, `decimals` and `max-supply` flags must be provided for a fungible faucet"
                    .to_string(),
            ));
        }

        let mut extra_components = Vec::new();
        for path in &self.extra_components {
            let bytes = fs::read(path)?;
            let template = AccountComponentTemplate::read_from_bytes(&bytes).map_err(|e| {
                CliError::AccountComponentError(
                    Box::new(e),
                    "failed to read account component template".into(),
                )
            })?;
            extra_components.push(template);
        }

        let decimals = self.decimals.expect("decimals must be provided");
        let token_symbol = self.token_symbol.clone().expect("token symbol must be provided");

        let key_pair = SecretKey::with_rng(client.rng());

        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let symbol = TokenSymbol::new(token_symbol.as_str()).map_err(CliError::Asset)?;
        let max_supply = Felt::try_from(
            self.max_supply.expect("max supply must be provided").to_le_bytes().as_slice(),
        )
        .expect("u64 can be safely converted to a field element");

        let anchor_block = client.get_latest_epoch_block().await?;

        let mut builder = AccountBuilder::new(init_seed)
            .anchor((&anchor_block).try_into().expect("anchor block should be valid"))
            .account_type(AccountType::FungibleFaucet)
            .storage_mode(self.storage_mode.into())
            .with_component(RpoFalcon512::new(key_pair.public_key()))
            .with_component(BasicFungibleFaucet::new(symbol, decimals, max_supply).map_err(
                |err| CliError::Account(err, "failed to create a faucet component".to_string()),
            )?);

        //add any extra component templates
        for component in process_component_templates(&extra_components)? {
            builder = builder.with_component(component);
        }

        let (new_account, seed) = builder
            .build()
            .map_err(|err| CliError::Account(err, "error building account".into()))?;

        client
            .add_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair), false)
            .await?;

        println!("Succesfully created new faucet.");
        println!(
            "To view account details execute `{CLIENT_BINARY_NAME} account -s {}`",
            new_account.id()
        );

        Ok(())
    }
}

// NEW WALLET
// ================================================================================================

/// Create a new wallet account.
#[derive(Debug, Parser, Clone)]
pub struct NewWalletCmd {
    /// Storage mode of the account.
    #[clap(value_enum, short, long, default_value_t = CliAccountStorageMode::Private)]
    pub storage_mode: CliAccountStorageMode,
    /// Defines if the account code is mutable (by default it isn't mutable).
    #[clap(short, long)]
    pub mutable: bool,
    /// Optional list of files specifying addional components to add to the account.
    #[clap(short, long)]
    pub extra_components: Vec<PathBuf>,
}

impl NewWalletCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), CliError> {
        let mut extra_components = Vec::new();
        for path in &self.extra_components {
            let bytes = fs::read(path)?;
            let template = AccountComponentTemplate::read_from_bytes(&bytes).map_err(|e| {
                CliError::AccountComponentError(
                    Box::new(e),
                    "failed to read account component template".into(),
                )
            })?;
            extra_components.push(template);
        }

        let key_pair = SecretKey::with_rng(client.rng());

        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let account_type = if self.mutable {
            AccountType::RegularAccountUpdatableCode
        } else {
            AccountType::RegularAccountImmutableCode
        };

        let anchor_block = client.get_latest_epoch_block().await?;

        let mut builder = AccountBuilder::new(init_seed)
            .anchor((&anchor_block).try_into().expect("anchor block should be valid"))
            .account_type(account_type)
            .storage_mode(self.storage_mode.into())
            .with_component(RpoFalcon512::new(key_pair.public_key()))
            .with_component(BasicWallet);

        //add any extra component templates
        for component in process_component_templates(&extra_components)? {
            builder = builder.with_component(component);
        }

        let (new_account, seed) = builder
            .build()
            .map_err(|err| CliError::Account(err, "failed to create a wallet".to_string()))?;

        client
            .add_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair), false)
            .await?;

        println!("Succesfully created new wallet.");
        println!(
            "To view account details execute `{CLIENT_BINARY_NAME} account -s {}`",
            new_account.id()
        );

        let (mut current_config, _) = load_config_file()?;
        maybe_set_default_account(&mut current_config, new_account.id())?;

        Ok(())
    }
}
