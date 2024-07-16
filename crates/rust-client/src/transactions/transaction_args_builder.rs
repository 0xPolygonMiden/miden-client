use alloc::string::String;

use miden_objects::{
    accounts::{AccountId, AuthSecretKey},
    assembly::{AssemblyError, ProgramAst},
    transaction::{TransactionArgs, TransactionScript},
    Felt,
};
use miden_tx::{
    auth::TransactionAuthenticator, DataStore, TransactionExecutor, TransactionExecutorError,
};

use super::{prepare_word, transaction_request::TransactionRequest};

pub struct TransactionArgsBuilder {
    auth: AuthSecretKey,
    tx_request: TransactionRequest,
    tx_script: Option<TransactionScript>,
}

impl TransactionArgsBuilder {
    pub fn new(auth: AuthSecretKey, tx_request: TransactionRequest) -> Self {
        Self { auth, tx_request, tx_script: None }
    }

    pub fn with_custom_script(mut self, tx_script: TransactionScript) -> Self {
        self.tx_script = Some(tx_script);
        self
    }

    pub fn with_native_output_notes<D: DataStore, A: TransactionAuthenticator>(
        mut self,
        tx_executor: &TransactionExecutor<D, A>,
    ) -> Result<Self, TransactionArgsBuilderError> {
        let script = format!("{} begin {} end", self.script_includes(), self.script_body()?);

        let program_ast = ProgramAst::parse(&script)
            .map_err(|err| TransactionArgsBuilderError::InvalidTransactionScript(err.into()))?;

        let tx_script = tx_executor
            .compile_tx_script(program_ast, vec![], vec![])
            .map_err(TransactionArgsBuilderError::TransactionExecutorError)?;
        self.tx_script = Some(tx_script);

        Ok(self)
    }

    pub fn build(self) -> TransactionArgs {
        let mut tx_args = TransactionArgs::new(
            self.tx_script,
            self.tx_request.get_note_args().into(),
            self.tx_request.advice_map().clone(),
        );

        let output_notes = self.tx_request.expected_output_notes().to_vec();
        tx_args.extend_expected_output_notes(output_notes);
        tx_args.extend_merkle_store(self.tx_request.merkle_store().inner_nodes());

        tx_args
    }

    fn script_includes(&self) -> String {
        let mut includes: String = String::from("use.miden::contracts::auth::basic->auth_tx\n");

        if self.tx_request.account_id().is_faucet() {
            includes.push_str("use.miden::contracts::faucets::basic_fungible->faucet\n");
        } else {
            includes.push_str("use.miden::contracts::wallets::basic->wallet\n");
        }

        includes
    }

    fn script_body(&self) -> Result<String, TransactionArgsBuilderError> {
        let account_id = self.tx_request.account_id();
        let mut body = String::new();

        for partial_note in self.tx_request.native_output_notes().iter() {
            if partial_note.metadata().sender() != account_id {
                return Err(TransactionArgsBuilderError::InvalidSenderAccount(
                    partial_note.metadata().sender(),
                ));
            }

            for asset in partial_note.assets().iter() {
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

                if self.tx_request.account_id().is_faucet() {
                    body.push_str(&format!(
                        "
                        push.{amount}
                        call.faucet::distribute dropw dropw
                        ",
                        amount = asset.unwrap_fungible().amount()
                    ));
                } else {
                    body.push_str(&format!(
                        "
                        push.{asset}
                        call.wallet::send_asset dropw dropw dropw dropw
                        ",
                        asset = prepare_word(&asset.into())
                    ));
                }
            }
        }

        match self.auth {
            AuthSecretKey::RpoFalcon512(_) => {
                body.push_str("call.auth_tx::auth_tx_rpo_falcon512\n");
            },
        }

        Ok(body)
    }
}

#[derive(Debug)]
pub enum TransactionArgsBuilderError {
    InvalidTransactionScript(AssemblyError),
    InvalidSenderAccount(AccountId),
    TransactionExecutorError(TransactionExecutorError),
}

impl core::fmt::Display for TransactionArgsBuilderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TransactionArgsBuilderError::InvalidTransactionScript(err) => {
                write!(f, "invalid transaction script: {}", err)
            },
            TransactionArgsBuilderError::InvalidSenderAccount(account_id) => {
                write!(f, "invalid sender account: {}", account_id)
            },
            TransactionArgsBuilderError::TransactionExecutorError(err) => {
                write!(f, "transaction executor error: {}", err)
            },
        }
    }
}
