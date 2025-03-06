use std::path::PathBuf;

use clap::Parser;
use miden_client::{crypto::FeltRng, Client, Felt, Word};
use miden_objects::{vm::AdviceInputs, Digest};

use crate::{errors::CliError, utils::get_input_acc_id_by_prefix_or_default};

// Exec COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "Execute the specified program against the specified account")]
pub struct ExecCmd {
    /// Account ID to use for the program execution
    #[clap(short = 'a', long = "account")]
    account_id: Option<String>,

    /// Path to script's source code to be executed
    #[clap(long, short)]
    script: String,
}

impl ExecCmd {
    pub async fn execute(&self, client: Client<impl FeltRng>) -> Result<(), CliError> {
        let script = PathBuf::from(&self.script);
        if !script.exists() {
            return Err(CliError::Exec(
                "Error with the program file".to_string().into(),
                format!("The program file at path {} does not exist", self.script),
            ));
        }

        let program = std::fs::read_to_string(script)?;

        let account_id =
            get_input_acc_id_by_prefix_or_default(&client, self.account_id.clone()).await?;

        let result =
            client.execute_program(account_id, &program, [], AdviceInputs::default()).await;
        match result {
            Ok(output_stack) => {
                println!("Program executed successfully");
                println!("Output stack:");
                print_stack(output_stack);
                Ok(())
            },
            Err(err) => Err(CliError::Exec(err.into(), "Error executing the program".to_string())),
        }
    }
}

fn print_stack(stack: [Felt; 16]) {
    for word_idx in 0..4 {
        let felts: [Felt; 4] = stack[word_idx * 4..word_idx * 4 + 4]
            .try_into()
            .expect("slice should have correct length");

        println!("- {:?}: {}", felts, Digest::new(Word::from(felts)));
    }
}
