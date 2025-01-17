use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{AccountId, AccountIdPrefix, AuthSecretKey},
    notes::PartialNote,
    transaction::TransactionScript,
    Felt, TransactionScriptError,
};
use miden_tx::TransactionExecutorError;
use thiserror::Error;

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
    /// - [TransactionScriptBuilderError::InvalidSenderAccount] if the sender of the note isn't the
    ///   account for which the script is being built.
    /// - [TransactionScriptBuilderError::FaucetNoteWithoutAsset] if the note created by the faucet
    ///   doesn't contain exactly one asset.
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
                    if partial_note.assets().num_assets() != 1 {
                        return Err(TransactionScriptBuilderError::FaucetNoteWithoutAsset);
                    }

                    // SAFETY: We checked that the note contains exactly one asset
                    let asset =
                        partial_note.assets().iter().next().expect("note should contain an asset");

                    if asset.faucet_id_prefix() != account_id.prefix() {
                        return Err(TransactionScriptBuilderError::InvalidAsset(
                            asset.faucet_id_prefix(),
                        ));
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
                        call.wallet::create_note\n",
                    );

                    for asset in partial_note.assets().iter() {
                        body.push_str(&format!(
                            "push.{asset}
                            call.wallet::move_asset_to_note dropw\n",
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
    /// The number of blocks in relation to the transaction's reference block after which the
    /// transaction will expire.
    expiration_delta: Option<u16>,
    /// Indicates if the script should be compiled in debug mode.
    in_debug_mode: bool,
}

impl TransactionScriptBuilder {
    pub fn new(
        account_capabilities: AccountCapabilities,
        expiration_delta: Option<u16>,
        in_debug_mode: bool,
    ) -> Self {
        Self {
            account_capabilities,
            expiration_delta,
            in_debug_mode,
        }
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

    /// Builds a simple authentication script for the transaction that doesn't send any notes.
    pub fn build_auth_script(&self) -> Result<TransactionScript, TransactionScriptBuilderError> {
        self.build_script_with_sections(vec![])
    }

    /// Builds a transaction script with the specified sections.
    ///
    /// The `sections` parameter is a vector of strings, where each string represents a distinct
    /// part of the script body. The script includes, authentication, and expiration sections are
    /// automatically added to the script.
    fn build_script_with_sections(
        &self,
        sections: Vec<String>,
    ) -> Result<TransactionScript, TransactionScriptBuilderError> {
        let script = format!(
            "{} begin {} {} {} end",
            self.script_includes(),
            self.script_expiration(),
            sections.join(" "),
            self.script_authentication()
        );

        let assembler = TransactionKernel::assembler().with_debug_mode(self.in_debug_mode);
        let tx_script = TransactionScript::compile(script, vec![], assembler)
            .map_err(TransactionScriptBuilderError::InvalidTransactionScript)?;

        Ok(tx_script)
    }

    /// Returns a string with the needed include instructions for the script.
    fn script_includes(&self) -> String {
        let mut includes = String::new();

        includes.push_str(self.account_capabilities.interfaces.script_includes());

        match self.account_capabilities.auth {
            AuthSecretKey::RpoFalcon512(_) => {
                includes.push_str("use.miden::contracts::auth::basic->auth_tx\n");
            },
        }

        if self.expiration_delta.is_some() {
            includes.push_str("use.miden::tx\n");
        }

        includes
    }

    /// Returns a string with the authentication procedure call for the script.
    fn script_authentication(&self) -> String {
        match self.account_capabilities.auth {
            AuthSecretKey::RpoFalcon512(_) => "call.auth_tx::auth_tx_rpo_falcon512\n".to_string(),
        }
    }

    /// Returns a string with the expiration delta update procedure call for the script.
    fn script_expiration(&self) -> String {
        if let Some(expiration_delta) = self.expiration_delta {
            format!("push.{} exec.tx::update_expiration_block_delta\n", expiration_delta)
        } else {
            String::new()
        }
    }
}

// TRANSACTION SCRIPT BUILDER ERROR
// ============================================================================================

/// Errors related to building a transaction script.
#[derive(Debug, Error)]
pub enum TransactionScriptBuilderError {
    #[error("invalid asset: {0}")]
    InvalidAsset(AccountIdPrefix),
    #[error("note created by the faucet doesn't contain exactly one asset")]
    FaucetNoteWithoutAsset,
    #[error("invalid transaction script")]
    InvalidTransactionScript(#[source] TransactionScriptError),
    #[error("invalid sender account: {0}")]
    InvalidSenderAccount(AccountId),
    #[error("transaction executor error")]
    TransactionExecutorError(#[source] TransactionExecutorError),
}
