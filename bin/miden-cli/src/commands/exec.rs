use std::path::PathBuf;

use clap::Parser;
use miden_client::{crypto::FeltRng, Client, Felt, Word};
use miden_objects::{vm::AdviceInputs, Digest};
use serde::{Deserialize, Serialize};

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

    /// Path to the inputs file
    ///
    /// The file should contain a JSON array of objects, where each object has two fields:
    /// - `key`: a 256-bit hexadecimal string representing a word to be used as a key for the input
    ///   entry
    /// - `values`: an array of 64-bit unsigned integers representing field elements to be used as
    ///   values for the input entry
    #[clap(long, short)]
    inputs: Option<String>,
}

impl ExecCmd {
    pub async fn execute(&self, client: Client<impl FeltRng>) -> Result<(), CliError> {
        let script = PathBuf::from(&self.script);
        if !script.exists() {
            return Err(CliError::Exec(
                "error with the program file".to_string().into(),
                format!("the program file at path {} does not exist", self.script),
            ));
        }

        let program = std::fs::read_to_string(script)?;

        let account_id =
            get_input_acc_id_by_prefix_or_default(&client, self.account_id.clone()).await?;

        let inputs = match &self.inputs {
            Some(input_file) => {
                let input_file = PathBuf::from(input_file);
                if !input_file.exists() {
                    return Err(CliError::Exec(
                        "error with the input file".to_string().into(),
                        format!("the input file at path {} does not exist", input_file.display()),
                    ));
                }

                let input_data = std::fs::read_to_string(input_file)?;
                deserialize_tx_inputs(&input_data)?
            },
            None => vec![],
        };

        let result = client
            .execute_program(account_id, &program, inputs, AdviceInputs::default())
            .await;

        match result {
            Ok(output_stack) => {
                println!("Program executed successfully");
                println!("Output stack:");
                print_stack(output_stack);
                Ok(())
            },
            Err(err) => Err(CliError::Exec(err.into(), "error executing the program".to_string())),
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

#[derive(Serialize, Deserialize)]
struct CliTxInput {
    key: String,
    values: Vec<u64>,
}

fn deserialize_tx_inputs(serialized: &str) -> Result<Vec<(Word, Vec<Felt>)>, CliError> {
    let cli_inputs: Vec<CliTxInput> = serde_json::from_str(serialized).map_err(|err| {
        CliError::Exec(
            "error deserializing transaction inputs".into(),
            format!("failed to parse input data: {err}"),
        )
    })?;
    cli_inputs
        .into_iter()
        .map(|input| {
            let word = Digest::try_from(input.key).map_err(|err| err.to_string())?.into();
            let felts = input.values.into_iter().map(Felt::new).collect();
            Ok((word, felts))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| CliError::Exec("error deserializing transaction inputs".into(), err))
}
