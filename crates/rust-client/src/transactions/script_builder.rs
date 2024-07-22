use alloc::string::String;

use miden_objects::{
    accounts::{AccountId, AuthSecretKey},
    assembly::{AssemblyError, ProgramAst},
    notes::PartialNote,
    transaction::TransactionScript,
    Felt,
};
use miden_tx::{
    auth::TransactionAuthenticator, DataStore, TransactionExecutor, TransactionExecutorError,
};

use super::prepare_word;

// ACCOUNT CAPABILITIES
// ============================================================================================
pub struct AccountCapabilities {
    pub account_id: AccountId,
    pub auth: AuthSecretKey,
    pub interfaces: AccountInterface,
}

pub enum AccountInterface {
    /// The account exposes procedures of the basic wallet.
    BasicWallet,
    /// The account is a fungible faucet and exposes procedures of the basic fungible faucet.
    BasicFungibleFaucet,
}

impl AccountInterface {
    /// Returns the script body that sends notes to the recipients.
    fn send_note_procedure(
        &self,
        account_id: AccountId,
        notes: &[PartialNote],
    ) -> Result<String, TransactionScriptBuilderError> {
        let mut body = String::new();

        for partial_note in notes.iter() {
            if partial_note.metadata().sender() != account_id {
                return Err(TransactionScriptBuilderError::InvalidSenderAccount(
                    partial_note.metadata().sender(),
                ));
            }

            if partial_note.assets().num_assets() != 1 {
                return Err(TransactionScriptBuilderError::InvalidAssetAmount(
                    partial_note.assets().num_assets(),
                ));
            }

            let asset = partial_note.assets().iter().next().expect("There should be an asset");

            body.push_str(&format!(
                "
                push.{recipient}
                push.{note_type}
                push.{aux}
                push.{tag}
                ",
                recipient = prepare_word(&partial_note.recipient_digest()),
                note_type = Felt::new(partial_note.metadata().note_type() as u64),
                aux = partial_note.metadata().aux(),
                tag = Felt::new(partial_note.metadata().tag().inner().into()),
            ));

            match self {
                AccountInterface::BasicFungibleFaucet => {
                    if asset.faucet_id() != account_id {
                        return Err(TransactionScriptBuilderError::InvalidAsset(asset.faucet_id()));
                    }

                    body.push_str(&format!(
                        "
                        push.{amount}
                        call.faucet::distribute dropw dropw
                        ",
                        amount = asset.unwrap_fungible().amount()
                    ));
                },
                AccountInterface::BasicWallet => {
                    body.push_str(&format!(
                        "
                        push.{asset}
                        call.wallet::send_asset dropw dropw dropw dropw
                        ",
                        asset = prepare_word(&asset.into())
                    ));
                },
            }
        }

        Ok(body)
    }

    fn script_includes(&self) -> &str {
        match self {
            AccountInterface::BasicWallet => "use.miden::contracts::wallets::basic->wallet\n",
            AccountInterface::BasicFungibleFaucet => {
                "use.miden::contracts::faucets::basic_fungible->faucet\n"
            },
        }
    }
}

// TRANSACTION SCRIPT BUILDER
// ============================================================================================
pub struct TransactionScriptBuilder {
    /// Capabilities of the account for which the script is being built. The capabilities
    /// specify the authentication method and the interfaces exposed by the account.
    account_capabilities: AccountCapabilities,
}

impl TransactionScriptBuilder {
    pub fn new(account_capabilities: AccountCapabilities) -> Self {
        Self { account_capabilities }
    }

    /// Builds a transaction script which sends the specified notes with the corresponding authentication.
    pub fn build_from_notes<D: DataStore, A: TransactionAuthenticator>(
        &self,
        tx_executor: &TransactionExecutor<D, A>,
        output_notes: &[PartialNote],
    ) -> Result<TransactionScript, TransactionScriptBuilderError> {
        let send_note_procedure = self
            .account_capabilities
            .interfaces
            .send_note_procedure(self.account_capabilities.account_id, output_notes)?;

        let script = format!(
            "{} begin {} {} end",
            self.script_includes(),
            send_note_procedure,
            self.script_authentication()
        );

        let program_ast = ProgramAst::parse(&script)
            .map_err(|err| TransactionScriptBuilderError::InvalidTransactionScript(err.into()))?;

        let tx_script = tx_executor
            .compile_tx_script(program_ast, vec![], vec![])
            .map_err(TransactionScriptBuilderError::TransactionExecutorError)?;

        Ok(tx_script)
    }

    /// Builds a simple authentication script for the account that doesn't send any notes.
    pub fn build_simple_authentication_script<D: DataStore, A: TransactionAuthenticator>(
        &self,
        tx_executor: &TransactionExecutor<D, A>,
    ) -> Result<TransactionScript, TransactionScriptBuilderError> {
        let script =
            format!("{} begin {} end", self.script_includes(), self.script_authentication());

        let program_ast = ProgramAst::parse(&script)
            .map_err(|err| TransactionScriptBuilderError::InvalidTransactionScript(err.into()))?;

        let tx_script = tx_executor
            .compile_tx_script(program_ast, vec![], vec![])
            .map_err(TransactionScriptBuilderError::TransactionExecutorError)?;

        Ok(tx_script)
    }

    fn script_includes(&self) -> String {
        let mut includes = String::new();

        includes.push_str(self.account_capabilities.interfaces.script_includes());

        match self.account_capabilities.auth {
            AuthSecretKey::RpoFalcon512(_) => {
                includes.push_str("use.miden::contracts::auth::basic->auth_tx\n");
            },
        }

        includes
    }

    fn script_authentication(&self) -> String {
        let mut body = String::new();

        match self.account_capabilities.auth {
            AuthSecretKey::RpoFalcon512(_) => {
                body.push_str("call.auth_tx::auth_tx_rpo_falcon512\n");
            },
        }

        body
    }
}

// TRANSACTION SCRIPT BUILDER ERROR
// ============================================================================================

#[derive(Debug)]
pub enum TransactionScriptBuilderError {
    InvalidAsset(AccountId),
    InvalidAssetAmount(usize),
    InvalidTransactionScript(AssemblyError),
    InvalidSenderAccount(AccountId),
    TransactionExecutorError(TransactionExecutorError),
}

impl core::fmt::Display for TransactionScriptBuilderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TransactionScriptBuilderError::InvalidAsset(account_id) => {
                write!(f, "Invalid asset: {}", account_id)
            },
            TransactionScriptBuilderError::InvalidAssetAmount(num_assets) => {
                write!(f, "Only notes with 1 type of asset are supported, but this note contains {} assets", num_assets)
            },
            TransactionScriptBuilderError::InvalidTransactionScript(err) => {
                write!(f, "Invalid transaction script: {}", err)
            },
            TransactionScriptBuilderError::InvalidSenderAccount(account_id) => {
                write!(f, "Invalid sender account: {}", account_id)
            },
            TransactionScriptBuilderError::TransactionExecutorError(err) => {
                write!(f, "Transaction executor error: {}", err)
            },
        }
    }
}
