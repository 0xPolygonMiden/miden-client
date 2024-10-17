use alloc::{
    string::{String, ToString},
    vec::Vec,
};

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
                    body.push_str(
                        "
                        call.wallet::create_note",
                    );

                    for asset in partial_note.assets().iter() {
                        body.push_str(&format!(
                            "
                        push.{asset}
                        call.wallet::move_asset_to_note dropw
                        ",
                            asset = prepare_word(&asset.into())
                        ))
                    }

                    body.push_str("dropw dropw dropw drop");
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
    expiration_delta: Option<u32>,
}

impl TransactionScriptBuilder {
    pub fn new(account_capabilities: AccountCapabilities, expiration_delta: Option<u32>) -> Self {
        Self { account_capabilities, expiration_delta }
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

        self.build_script_with_sections(vec![send_note_procedure])
    }

    /// Builds a simple authentication script for the account that doesn't send any notes.
    pub fn build_auth_script(&self) -> Result<TransactionScript, TransactionScriptBuilderError> {
        self.build_script_with_sections(vec![])
    }

    fn build_script_with_sections(
        &self,
        sections: Vec<String>,
    ) -> Result<TransactionScript, TransactionScriptBuilderError> {
        let script = format!(
            "{} begin {} {} {} end",
            self.script_includes(),
            sections.join(" "),
            self.script_expiration(),
            self.script_authentication()
        );

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

        if self.expiration_delta.is_some() {
            includes.push_str("use.miden::kernels::tx::update_expiration_block_num\n");
        }

        includes
    }

    fn script_authentication(&self) -> String {
        match self.account_capabilities.auth {
            AuthSecretKey::RpoFalcon512(_) => "call.auth_tx::auth_tx_rpo_falcon512\n".to_string(),
        }
    }

    fn script_expiration(&self) -> String {
        if let Some(expiration_delta) = self.expiration_delta {
            format!("push.{} call.update_expiration_block_num", expiration_delta)
        } else {
            String::new()
        }
    }
}

// TRANSACTION SCRIPT BUILDER ERROR
// ============================================================================================

/// Errors related to building a transaction script.
#[derive(Debug)]
pub enum TransactionScriptBuilderError {
    InvalidAsset(AccountId),
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
