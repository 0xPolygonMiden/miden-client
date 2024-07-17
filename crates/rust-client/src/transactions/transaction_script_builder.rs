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

pub struct AccountSpecification {
    pub account_id: AccountId,
    pub auth: AuthSecretKey,
    pub capabilities: AccountCapabilities,
}

pub enum AccountCapabilities {
    /// The account exposes procedures of the basic wallet.
    BasicWallet,
    /// The account is a fungible faucet and exposes procedures of the basic fungible faucet.
    BasicFungibleFaucet,
}

impl AccountCapabilities {
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

            if partial_note.assets().num_assets() > 1 {
                return Err(TransactionScriptBuilderError::TooManyAssets(
                    partial_note.assets().num_assets(),
                ));
            }

            let asset = partial_note.assets().iter().next();
            if let Some(asset) = asset {
                if let AccountCapabilities::BasicFungibleFaucet = self {
                    if asset.faucet_id() != account_id {
                        return Err(TransactionScriptBuilderError::InvalidAsset(asset.faucet_id()));
                    }
                }

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
                    AccountCapabilities::BasicFungibleFaucet => {
                        body.push_str(&format!(
                            "
                            push.{amount}
                            call.faucet::distribute dropw dropw
                            ",
                            amount = asset.unwrap_fungible().amount()
                        ));
                    },
                    AccountCapabilities::BasicWallet => {
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
        }

        Ok(body)
    }

    fn script_includes(&self) -> &str {
        match self {
            AccountCapabilities::BasicWallet => "use.miden::contracts::wallets::basic->wallet\n",
            AccountCapabilities::BasicFungibleFaucet => {
                "use.miden::contracts::faucets::basic_fungible->faucet\n"
            },
        }
    }
}

pub struct TransactionScriptBuilder {
    account_spec: AccountSpecification,
}

impl TransactionScriptBuilder {
    pub fn new(account_spec: AccountSpecification) -> Self {
        Self { account_spec }
    }

    pub fn build_from_notes<D: DataStore, A: TransactionAuthenticator>(
        &self,
        tx_executor: &TransactionExecutor<D, A>,
        output_notes: &[PartialNote],
    ) -> Result<TransactionScript, TransactionScriptBuilderError> {
        let script =
            format!("{} begin {} end", self.script_includes(), self.script_body(output_notes)?);

        let program_ast = ProgramAst::parse(&script)
            .map_err(|err| TransactionScriptBuilderError::InvalidTransactionScript(err.into()))?;

        let tx_script = tx_executor
            .compile_tx_script(program_ast, vec![], vec![])
            .map_err(TransactionScriptBuilderError::TransactionExecutorError)?;

        Ok(tx_script)
    }

    fn script_includes(&self) -> String {
        let mut includes = String::new();

        includes.push_str(self.account_spec.capabilities.script_includes());

        match self.account_spec.auth {
            AuthSecretKey::RpoFalcon512(_) => {
                includes.push_str("use.miden::contracts::auth::basic->auth_tx\n");
            },
        }

        includes
    }

    fn script_body(
        &self,
        output_notes: &[PartialNote],
    ) -> Result<String, TransactionScriptBuilderError> {
        let mut body = String::new();

        body.push_str(
            &self
                .account_spec
                .capabilities
                .send_note_procedure(self.account_spec.account_id, output_notes)?,
        );

        match self.account_spec.auth {
            AuthSecretKey::RpoFalcon512(_) => {
                body.push_str("call.auth_tx::auth_tx_rpo_falcon512\n");
            },
        }

        Ok(body)
    }
}

#[derive(Debug)]
pub enum TransactionScriptBuilderError {
    InvalidAsset(AccountId),
    InvalidTransactionScript(AssemblyError),
    InvalidSenderAccount(AccountId),
    TransactionExecutorError(TransactionExecutorError),
    TooManyAssets(usize),
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
            TransactionScriptBuilderError::TooManyAssets(num_assets) => {
                write!(f, "Only notes with 0 or 1 different assets are supported, but this note contains {} assets", num_assets)
            },
        }
    }
}
