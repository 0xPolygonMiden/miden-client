use alloc::string::String;

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{AccountId, AuthSecretKey},
    notes::PartialNote,
    transaction::TransactionScript,
    Felt, TransactionScriptError,
};
use miden_tx::TransactionExecutorError;

use super::prepare_word;

// ACCOUNT CAPABILITIES
// ============================================================================================
pub(crate) struct AccountCapabilities {
    pub account_id: AccountId,
    pub auth: AuthSecretKey,
    pub interfaces: AccountInterface,
}

pub(crate) enum AccountInterface {
    /// The account exposes procedures of the basic wallet.
    BasicWallet,
    /// The account is a fungible faucet and exposes procedures of the basic fungible faucet.
    BasicFungibleFaucet,
}

impl AccountInterface {
    /// Returns the script body that sends notes to the recipients.
    ///
    /// Errors:
    /// - [TransactionScriptBuilderError::InvalidSenderAccount] if the sender of the note is not the
    ///   account for which the script is being built.
    /// - [TransactionScriptBuilderError::InvalidAssetAmount] if the note does not contain exactly
    ///   one asset.
    /// - [TransactionScriptBuilderError::InvalidAsset] if a faucet tries to distribute an asset
    ///   with a different faucet ID.
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
                push.{execution_hint}
                push.{note_type}
                push.{aux}
                push.{tag}
                ",
                recipient = prepare_word(&partial_note.recipient_digest()),
                note_type = Felt::from(partial_note.metadata().note_type()),
                execution_hint = Felt::from(partial_note.metadata().execution_hint()),
                aux = partial_note.metadata().aux(),
                tag = Felt::from(partial_note.metadata().tag()),
            ));

            match self {
                AccountInterface::BasicFungibleFaucet => {
                    if asset.faucet_id() != account_id {
                        return Err(TransactionScriptBuilderError::InvalidAsset(asset.faucet_id()));
                    }

                    body.push_str(&format!(
                        "
                        push.{amount}
                        call.faucet::distribute dropw dropw drop
                        ",
                        amount = asset.unwrap_fungible().amount()
                    ));
                },
                AccountInterface::BasicWallet => {
                    body.push_str(&format!(
                        "
                        push.{asset}
                        call.wallet::send_asset dropw dropw dropw dropw drop
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
pub(crate) struct TransactionScriptBuilder {
    /// Capabilities of the account for which the script is being built. The capabilities
    /// specify the authentication method and the interfaces exposed by the account.
    account_capabilities: AccountCapabilities,
}

impl TransactionScriptBuilder {
    pub fn new(account_capabilities: AccountCapabilities) -> Self {
        Self { account_capabilities }
    }

    /// Builds a transaction script which sends the specified notes with the corresponding
    /// authentication.
    pub fn build_send_notes_script(
        &self,
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

        let tx_script = TransactionScript::compile(script, vec![], TransactionKernel::assembler())
            .map_err(TransactionScriptBuilderError::InvalidTransactionScript)?;

        Ok(tx_script)
    }

    /// Builds a simple authentication script for the account that doesn't send any notes.
    pub fn build_auth_script(&self) -> Result<TransactionScript, TransactionScriptBuilderError> {
        let script =
            format!("{} begin {} end", self.script_includes(), self.script_authentication());

        let tx_script = TransactionScript::compile(script, vec![], TransactionKernel::assembler())
            .map_err(TransactionScriptBuilderError::InvalidTransactionScript)?;

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

/// Errors related to building a transaction script.
#[derive(Debug)]
pub enum TransactionScriptBuilderError {
    InvalidAsset(AccountId),
    InvalidAssetAmount(usize),
    InvalidTransactionScript(TransactionScriptError),
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
