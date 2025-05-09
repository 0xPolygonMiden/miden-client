use std::{collections::BTreeSet, path::PathBuf};

use clap::Parser;
use miden_client::{Client, Felt, Word};
use miden_objects::{Digest, vm::AdviceInputs};
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{errors::CliError, utils::get_input_acc_id_by_prefix_or_default};

// EXEC COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[command(about = "Execute the specified program against the specified account")]
pub struct ExecCmd {
    /// Account ID to use for the program execution
    #[arg(short = 'a', long = "account")]
    account_id: Option<String>,

    /// Path to script's source code to be executed
    #[arg(long, short)]
    script_path: String,

    #[rustfmt::skip]
    #[allow(clippy::doc_link_with_quotes)]
    /// Path to the inputs file. This file will be used as inputs to the VM's advice map.
    ///
    /// The file should contain a TOML array of inline tables, where each table has two fields:
    /// - `key`: a 256-bit hexadecimal string representing a word to be used as a key for the input
    ///   entry. The hexadecimal value must be prefixed with 0x.
    /// - `values`: an array of 64-bit unsigned integers representing field elements to be used as
    ///   values for the input entry. Each integer must be written as a separate string, within
    ///   double quotes.
    ///
    /// The input file should contain a TOML table called `inputs`, as in the following example:
    ///    inputs = [
    ///        { key = "0x0000001000000000000000000000000000000000000000000000000000000000", values = ["13", "9"]},
    ///        { key = "0x0000000000000000000000000000000000000000000000000000000000000000" , values = ["1", "2"]},
    ///    ]
    #[arg(long, short)]
    inputs_path: Option<String>,

    /// Print the output stack grouped into words
    #[arg(long, default_value_t = false)]
    hex_words: bool,
}

impl ExecCmd {
    pub async fn execute(&self, mut client: Client) -> Result<(), CliError> {
        let script_path = PathBuf::from(&self.script_path);
        if !script_path.exists() {
            return Err(CliError::Exec(
                "error with the program file".to_string().into(),
                format!("the program file at path {} does not exist", self.script_path),
            ));
        }

        let program = std::fs::read_to_string(script_path)?;

        let account_id =
            get_input_acc_id_by_prefix_or_default(&client, self.account_id.clone()).await?;

        let inputs = match &self.inputs_path {
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

// INPUT FILE PROCESSING
// ===============================================================================================

/// Struct that holds a single key-values pair from the provided file inputs file. These will be
/// aggregated in the [`CliTxInputs`] struct
#[derive(Serialize, Deserialize)]
struct CliTxInput {
    key: String,
    #[serde(deserialize_with = "string_to_u64")]
    values: Vec<u64>,
}

/// Struct that holds every key-values pair present in the provided inputs file. This struct can be
/// iterated on to access the different keys.
#[derive(Serialize, Deserialize)]
struct CliTxInputs {
    inputs: Vec<CliTxInput>,
}

impl IntoIterator for CliTxInputs {
    type Item = CliTxInput;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.inputs.into_iter()
    }
}

/// Since the toml crate has problems parsing u64 values (see
/// [issue](https://github.com/toml-rs/toml/issues/705), we store the values as Strings. Then, when
/// deserializing, we turn those Strings to u64 in order to then turn them to Felts.
fn string_to_u64<'de, D>(deserializer: D) -> Result<Vec<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<String>::deserialize(deserializer)?
        .into_iter()
        .map(|a| a.parse::<u64>())
        .collect::<Result<Vec<u64>, _>>()
        .map_err(|_| {
            de::Error::custom(
                "invalid type: expected u64 in between parentheses. For example: values = [\"13\", \"9\"]",
            )
        })
}

fn deserialize_tx_inputs(serialized: &str) -> Result<Vec<(Word, Vec<Felt>)>, CliError> {
    let cli_inputs: CliTxInputs = toml::from_str(serialized).map_err(|err| {
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
