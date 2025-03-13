use std::{collections::BTreeSet, path::PathBuf};

use clap::Parser;
use miden_client::{Client, Felt, Word, crypto::FeltRng};
use miden_objects::{Digest, vm::AdviceInputs};
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

    /// Print the output stack grouped into words
    #[clap(long, default_value_t = false)]
    hex_words: bool,
}

impl ExecCmd {
    pub async fn execute(&self, mut client: Client<impl FeltRng>) -> Result<(), CliError> {
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

        let tx_script = client.compile_tx_script(inputs, &program)?;

        let result = client
            .execute_program(account_id, tx_script, AdviceInputs::default(), BTreeSet::new())
            .await;

        match result {
            Ok(output_stack) => {
                println!("Program executed successfully");
                println!("Output stack:");
                self.print_stack(output_stack);
                Ok(())
            },
            Err(err) => Err(CliError::Exec(err.into(), "error executing the program".to_string())),
        }
    }

    /// Print the output stack in a human-readable format
    fn print_stack(&self, stack: [Felt; 16]) {
        if self.hex_words {
            for i in 0..4 {
                let word_idx = i * 4;
                let word_idx_end = word_idx + 3;

                if word_idx == 12 {
                    print!("└── {word_idx:2} - {word_idx_end:2}: ");
                } else {
                    print!("├── {word_idx:2} - {word_idx_end:2}: ");
                }

                let word: [Felt; 4] =
                    stack[word_idx..=word_idx_end].try_into().expect("Length should be 4");

                println!("{:?} ({})", word, Digest::from(word));
            }
        } else {
            for (i, value) in stack.iter().enumerate() {
                if i == 15 {
                    println!("└── {i:2}: {value}");
                } else {
                    println!("├── {i:2}: {value}");
                }
            }
        }
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
