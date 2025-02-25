use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

use clap::{Parser, ValueEnum};
use miden_client::{
    account::{
        component::COMPONENT_TEMPLATE_EXTENSION, Account, AccountBuilder, AccountStorageMode,
        AccountType,
    },
    auth::AuthSecretKey,
    authenticator::keystore::{FilesystemKeyStore, KeyStore},
    crypto::{FeltRng, SecretKey},
    utils::Deserializable,
    Client, Word,
};
use miden_lib::account::auth::RpoFalcon512;
use miden_objects::account::{
    AccountComponent, AccountComponentTemplate, InitStorageData, StorageValueName,
};

use crate::{
    commands::account::maybe_set_default_account, errors::CliError, utils::load_config_file,
    CLIENT_BINARY_NAME,
};

// CLI TYPES
// ================================================================================================

/// Mirror enum for [AccountStorageMode] that enables parsing for CLI commands.
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

/// Mirror enum for [AccountType] that enables parsing for CLI commands.
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
    /// Optional file path to a TOML file containing a list of key/values used for initializing
    /// storage. Each of these keys should map to the templated storage values within the passed
    /// list of component templates. The user will be prompted to provide values for any keys not
    /// present in the init storage data file.
    #[clap(short, long)]
    pub init_storage_data_path: Option<PathBuf>,
}

impl NewWalletCmd {
    pub async fn execute(
        &self,
        mut client: Client<impl FeltRng>,
        keystore: FilesystemKeyStore,
    ) -> Result<(), CliError> {
        // Load extra component templates using the helper.
        let extra_components = load_component_templates(&self.extra_components)?;
        let init_storage_data = load_init_storage_data(self.init_storage_data_path.clone())?;

        let key_pair = SecretKey::with_rng(client.rng());

        // Choose account type based on mutability.
        let account_type = if self.mutable {
            AccountType::RegularAccountUpdatableCode
        } else {
            AccountType::RegularAccountImmutableCode
        };

        let (new_account, seed) = build_account(
            &mut client,
            account_type,
            self.storage_mode.into(),
            &key_pair,
            &extra_components,
            &init_storage_data,
        )
        .await?;

        keystore
            .add_key(&AuthSecretKey::RpoFalcon512(key_pair))
            .map_err(CliError::KeyStore)?;

        client.add_account(&new_account, Some(seed), false).await?;

        println!("Succesfully created new wallet.");
        println!(
            "To view account details execute {CLIENT_BINARY_NAME} account -s {}",
            new_account.id()
        );

        let (mut current_config, _) = load_config_file()?;
        maybe_set_default_account(&mut current_config, new_account.id())?;

        Ok(())
    }
}

// NEW ACCOUNT
// ================================================================================================

/// Creates a new account and saves it locally.
///
/// An account may comprise one or more components, each with its own storage and distinct
/// functionality.
#[derive(Debug, Parser, Clone)]
pub struct NewAccountCmd {
    /// Storage mode of the account.
    #[clap(value_enum, short, long, default_value_t = CliAccountStorageMode::Private)]
    pub storage_mode: CliAccountStorageMode,
    /// Account type to create.
    #[clap(long, value_enum)]
    pub account_type: CliAccountType,
    /// Optional list of files specifying additional component template files to add to the
    /// account.
    #[clap(short, long)]
    pub component_templates: Vec<PathBuf>,
    /// Optional file path to a TOML file containing a list of key/values used for initializing
    /// storage. Each of these keys should map to the templated storage values within the passed
    /// list of component templates. The user will be prompted to provide values for any keys not
    /// present in the init storage data file.
    #[clap(short, long)]
    pub init_storage_data_path: Option<PathBuf>,
}

impl NewAccountCmd {
    pub async fn execute(
        &self,
        mut client: Client<impl FeltRng>,
        keystore: FilesystemKeyStore,
    ) -> Result<(), CliError> {
        // Load component templates using the helper.
        let component_templates = load_component_templates(&self.component_templates)?;

        if component_templates.is_empty() {
            return Err(CliError::InvalidArgument(
                "account must contain one or more components".into(),
            ));
        }

        let init_storage_data = load_init_storage_data(self.init_storage_data_path.clone())?;

        let key_pair = SecretKey::with_rng(client.rng());

        let (new_account, seed) = build_account(
            &mut client,
            self.account_type.into(),
            self.storage_mode.into(),
            &key_pair,
            &component_templates,
            &init_storage_data,
        )
        .await?;

        keystore
            .add_key(&AuthSecretKey::RpoFalcon512(key_pair))
            .map_err(CliError::KeyStore)?;

        client.add_account(&new_account, Some(seed), false).await?;

        println!("Succesfully created new account.");
        println!(
            "To view account details execute {CLIENT_BINARY_NAME} account -s {}",
            new_account.id()
        );

        Ok(())
    }
}

// HELPERS
// ================================================================================================

/// Reads component templates from the given file paths.
// TODO: IO errors should have more context
fn load_component_templates(paths: &[PathBuf]) -> Result<Vec<AccountComponentTemplate>, CliError> {
    let (cli_config, _) = load_config_file()?;
    let components_base_dir = &cli_config.component_template_directory;
    let mut templates = Vec::new();
    for path in paths {
        // Set extension to COMPONENT_TEMPLATE_EXTENSION in case user did not
        let path = if path.extension().is_none() {
            path.with_extension(COMPONENT_TEMPLATE_EXTENSION)
        } else {
            path.clone()
        };
        let bytes = fs::read(components_base_dir.join(path))?;
        let template = AccountComponentTemplate::read_from_bytes(&bytes).map_err(|e| {
            CliError::AccountComponentError(
                Box::new(e),
                "failed to read account component template".into(),
            )
        })?;
        templates.push(template);
    }
    Ok(templates)
}

/// Loads the initialization storage data from an optional TOML file.
/// If None is passed, an empty object is returned.
fn load_init_storage_data(path: Option<PathBuf>) -> Result<InitStorageData, CliError> {
    if let Some(path) = path {
        let mut contents = String::new();
        let _ = File::open(path).and_then(|mut f| f.read_to_string(&mut contents))?;
        // TODO: Remove unwrap
        Ok(InitStorageData::from_toml(&contents).unwrap())
    } else {
        Ok(InitStorageData::default())
    }
}

/// Helper function to create the seed, initialize the account builder, add the given components,
/// and build the account.
async fn build_account(
    client: &mut Client<impl FeltRng>,
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    key_pair: &SecretKey,
    component_templates: &[AccountComponentTemplate],
    init_storage_data: &InitStorageData,
) -> Result<(Account, Word), CliError> {
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let anchor_block = client.get_latest_epoch_block().await?;
    let mut builder = AccountBuilder::new(init_seed)
        .anchor((&anchor_block).try_into().expect("anchor block should be valid"))
        .account_type(account_type)
        .storage_mode(storage_mode)
        // TODO: Forcing an auth component for simplicity for now
        .with_component(RpoFalcon512::new(key_pair.public_key()));

    // Add each processed component template
    for component in process_component_templates(component_templates, init_storage_data)? {
        builder = builder.with_component(component);
    }

    builder
        .build()
        .map_err(|err| CliError::Account(err, "failed to build account".into()))
}

/// Helper function to process extra component templates.
/// It reads user input for each placeholder in a component template.
fn process_component_templates(
    extra_components: &[AccountComponentTemplate],
    file_init_storage_data: &InitStorageData,
) -> Result<Vec<AccountComponent>, CliError> {
    let mut account_components = vec![];
    for component_template in extra_components {
        let mut init_storage_data: BTreeMap<StorageValueName, String> =
            file_init_storage_data.placeholders().clone();
        for (placeholder_key, placeholder_type) in
            component_template.metadata().get_placeholder_requirements()
        {
            if init_storage_data.contains_key(&placeholder_key) {
                // The use provided it through the TOML file, so we can skip it
                continue;
            }

            let description = placeholder_type.description.unwrap_or("[No description]".into());
            print!(
                "Enter value for '{placeholder_key}' - {description} (type: {}): ",
                placeholder_type.r#type
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
