use std::{collections::BTreeMap, path::PathBuf};

use miden_client::accounts::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FaucetDetails {
    pub id: String,
    pub decimals: u8,
}
pub struct TokenSymbolMappings {
    mappings_file: PathBuf,
}

impl TokenSymbolMappings {
    pub fn new(mappings_file: PathBuf) -> Self {
        Self { mappings_file }
    }

    pub fn get_token_symbol(&self, faucet_id: &AccountId) -> Result<String, String> {
        let mappings = self.load_mappings()?;
        mappings
            .iter()
            .find(|(_, faucet)| faucet.id == faucet_id.to_hex())
            .ok_or_else(|| format!("Faucet ID '{}' was not found in the mappings", faucet_id))
            .map(|(symbol, _)| symbol.clone())
    }

    pub fn set_token_symbol(
        &self,
        faucet_id: AccountId,
        decimals: u8,
        token_symbol: String,
    ) -> Result<(), String> {
        let mut mappings = self.load_mappings()?;
        let faucet_id = faucet_id.to_hex();
        if let Some(details) = mappings.get(&token_symbol) {
            return Err(format!(
                "Token symbol '{}' is already defined for faucet ID '{}', it will not be added as a mapping for faucet ID '{}'",
                token_symbol, details.id, faucet_id
            ));
        }

        if let Some((existing_token_symbol, _)) = mappings.iter().find(|(_, faucet)| faucet.id == faucet_id)
        {
            return Err(format!(
                "Faucet ID '{}' is already defined for token symbol '{}', it will not be added as a mapping for token symbol '{}'",
                faucet_id, existing_token_symbol, token_symbol
            ));
        }

        mappings.insert(token_symbol, FaucetDetails { id: faucet_id, decimals });
        self.save_mappings(&mappings)
    }

    pub fn get_faucet_id(&self, token_symbol: &String) -> Result<AccountId, String> {
        let mappings = self.load_mappings()?;
        let faucet_id = mappings
            .get(token_symbol)
            .map(|faucet| faucet.id.clone())
            .ok_or_else(|| format!("Token symbol '{}' was not found in the mappings", token_symbol))?;

        AccountId::from_hex(&faucet_id).map_err(|err| format!("Failed to parse faucet ID: {}", err))
    }

    fn load_mappings(&self) -> Result<BTreeMap<String, FaucetDetails>, String> {
        let mappings = match std::fs::read_to_string(&self.mappings_file) {
            Ok(content) => match toml::from_str(&content) {
                Ok(mappings) => mappings,
                Err(err) => return Err(format!("Failed to parse mappings file: {}", err)),
            },
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    return Err(format!("Failed to read mappings file: {}", err));
                }
                BTreeMap::new()
            },
        };

        Ok(mappings)
    }

    fn save_mappings(&self, mappings: &BTreeMap<String, FaucetDetails>) -> Result<(), String> {
        let content = toml_edit::ser::to_string(mappings)
            .map_err(|err| format!("Failed to serialize mappings: {}", err))?;

        std::fs::write(&self.mappings_file, content)
            .map_err(|err| format!("Failed to write mappings file: {}", err))
    }
}
