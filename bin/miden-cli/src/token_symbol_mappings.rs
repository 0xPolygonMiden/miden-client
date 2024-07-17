use std::{collections::BTreeMap, path::PathBuf};

use miden_client::accounts::AccountId;

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
            .get(&faucet_id.to_string())
            .cloned()
            .ok_or_else(|| format!("Faucet ID '{}' is not defined", faucet_id))
    }

    pub fn set_token_symbol(
        &self,
        faucet_id: &AccountId,
        token_symbol: String,
    ) -> Result<(), String> {
        let mut mappings = self.load_mappings().unwrap_or_default();
        mappings.insert(faucet_id.to_string(), token_symbol);
        self.save_mappings(&mappings)
    }

    pub fn get_faucet_id(&self, token_symbol: &String) -> Result<AccountId, String> {
        let mappings = self.load_mappings()?;
        let matches: Vec<String> = mappings
            .into_iter()
            .filter(|(_, symbol)| symbol == token_symbol)
            .map(|(faucet_id, _)| faucet_id)
            .collect();

        if matches.len() > 1 {
            return Err(format!(
                "Multiple faucet IDs found for token symbol '{}': {:?}",
                token_symbol, matches
            ));
        }

        let faucet_id = matches
            .first()
            .ok_or_else(|| format!("Token symbol '{}' is not defined", token_symbol))?;
        AccountId::from_hex(faucet_id).map_err(|err| format!("Invalid faucet ID: {}", err))
    }

    fn load_mappings(&self) -> Result<BTreeMap<String, String>, String> {
        let mappings = match std::fs::read_to_string(&self.mappings_file) {
            Ok(content) => match toml::from_str(&content) {
                Ok(mappings) => mappings,
                Err(err) => return Err(format!("Failed to parse mappings file: {}", err)),
            },
            Err(err) => {
                return Err(format!("Failed to read mappings file: {}", err));
            },
        };

        Ok(mappings)
    }

    fn save_mappings(&self, mappings: &BTreeMap<String, String>) -> Result<(), String> {
        let content = toml::to_string_pretty(mappings)
            .map_err(|err| format!("Failed to serialize mappings: {}", err))?;

        std::fs::write(&self.mappings_file, content)
            .map_err(|err| format!("Failed to write mappings file: {}", err))
    }
}
