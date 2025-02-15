use std::{collections::BTreeMap, fs, io::Write, path::PathBuf};

use clap::{Parser, ValueEnum};
use miden_client::{
    account::{AccountBuilder, AccountStorageMode, AccountType},
    auth::AuthSecretKey,
    crypto::{FeltRng, SecretKey},
    utils::Deserializable,
    Client,
};
use miden_lib::account::{auth::RpoFalcon512, wallets::BasicWallet};
use miden_objects::account::{AccountComponent, AccountComponentTemplate, InitStorageData};

use crate::{
    commands::account::maybe_set_default_account, errors::CliError, utils::load_config_file,
    CLIENT_BINARY_NAME,
};

const FAUCET_TEMPLATE_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/templates/faucet.mct"));

// CLI TYPES
// ================================================================================================

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum CliAccountType {
    FungibleFaucet,
    NonFungibleFaucet,
    RegularAccountImmutableCode,
    RegularAccountUpdatableCode,
}

impl From<CliAccountType> for AccountType {
    fn from(cli_type: CliAccountType) -> Self {
        match cli_type {
            CliAccountType::FungibleFaucet => AccountType::FungibleFaucet,
            CliAccountType::NonFungibleFaucet => AccountType::NonFungibleFaucet,
            CliAccountType::RegularAccountImmutableCode => AccountType::RegularAccountImmutableCode,
            CliAccountType::RegularAccountUpdatableCode => AccountType::RegularAccountUpdatableCode,
        }
    }
}

// NEW WALLET
// ================================================================================================

/// Creates a new wallet account and store it locally.
///
/// A wallet account exposes functionality to sign transactions and
/// manage asset transfers. Additionally, more component templates can be added by specifying
/// a list of component template files.
#[derive(Debug, Parser, Clone)]
pub struct NewWalletCmd {
    /// Storage mode of the account.
    #[clap(value_enum, short, long, default_value_t = CliAccountStorageMode::Private)]
    pub storage_mode: CliAccountStorageMode,
    /// Defines if the account code is mutable (by default it isn't mutable).
    #[clap(short, long)]
    pub mutable: bool,
    /// Optional list of files specifying additional components to add to the account.
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

        // Add any extra component templates
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
            component_template.metadata().get_placeholder_requirements()
        {
            let description = placeholder_type.description.unwrap_or("[No description]".into());
            print!(
                "Enter value for placeholder '{placeholder_key}' - {description} (type: {}): ",
                placeholder_type.r#type.to_string()
            );
            std::io::stdout().flush()?;

            let mut input_value = String::new();
            std::io::stdin().read_line(&mut input_value)?;
            let input_value = input_value.trim();
            init_storage_data.insert(placeholder_key, input_value.to_string());
        }

        let component = AccountComponent::from_template(
            component_template,
            &InitStorageData::new(init_storage_data),
        )
        .map_err(|e| CliError::Account(e, "error instantiating component from template".into()))?;

        account_components.push(component);
    }

    Ok(account_components)
}

// NEW ACCOUNT
// ================================================================================================

/// Creates a new account and saves it locally.
///
/// An account may comprise one or components, each with its own storage and distinct
/// functionality.
#[derive(Debug, Parser, Clone)]
pub struct NewAccountCmd {
    /// Storage mode of the account.
    #[clap(value_enum, short, long, default_value_t = CliAccountStorageMode::Private)]
    pub storage_mode: CliAccountStorageMode,
    /// Account type to create.
    #[clap(long, value_enum)]
    pub account_type: CliAccountType,
    /// If set, add the fungible faucet component template. The fungible faucet component exposes
    /// functionality for distributing assets and burning them. Only accounts of type
    /// [`CliAccountType::FungibleFaucet`] can make use of this component.
    #[clap(long)]
    pub fungible_faucet_component: bool,
    /// Optional list of files specifying additional component template files to add to the
    /// account.
    #[clap(short, long)]
    pub component_templates: Vec<PathBuf>,
}

impl NewAccountCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), CliError> {
        let mut component_templates = Vec::new();
        for path in &self.component_templates {
            let bytes = fs::read(path)?;
            let template = AccountComponentTemplate::read_from_bytes(&bytes).map_err(|e| {
                CliError::AccountComponentError(
                    Box::new(e),
                    "failed to read account component template".into(),
                )
            })?;
            component_templates.push(template);
        }

        // If --fungible-faucet-component is set, add it to the list
        if self.fungible_faucet_component {
            let template = AccountComponentTemplate::read_from_bytes(FAUCET_TEMPLATE_BYTES)
                .map_err(|e| {
                    CliError::AccountComponentError(
                        Box::new(e),
                        "failed to read fungible faucet component template".into(),
                    )
                })?;
            component_templates.push(template);
        }

        if component_templates.is_empty() {
            return Err(CliError::InvalidArgument(
                "account must contain one or more components".into(),
            ));
        }

        let processed_components = process_component_templates(&component_templates)?;

        let key_pair = SecretKey::with_rng(client.rng());

        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        // Use the provided account type.
        let anchor_block = client.get_latest_epoch_block().await?;

        let mut builder = AccountBuilder::new(init_seed)
            .anchor((&anchor_block).try_into().expect("anchor block should be valid"))
            .account_type(self.account_type.into())
            .storage_mode(self.storage_mode.into())
            // TODO: We should probably not force this
            .with_component(RpoFalcon512::new(key_pair.public_key()));

        // add any processed extra components
        for component in processed_components {
            builder = builder.with_component(component);
        }

        let (new_account, seed) = builder
            .build()
            .map_err(|err| CliError::Account(err, "error building account".into()))?;

        client
            .add_account(&new_account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair), false)
            .await?;

        println!("Succesfully created new account. {}", new_account.account_type());
        println!(
            "To view account details execute `{CLIENT_BINARY_NAME} account -s {}`",
            new_account.id()
        );

        Ok(())
    }
}
