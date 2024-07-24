use std::{collections::BTreeMap, path::PathBuf};

use miden_client::accounts::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FaucetDetails {
    pub id: String,
    pub decimals: u8,
}
pub struct TokenSymbolMap {
    mappings_file: PathBuf,
}

impl TokenSymbolMap {
    pub fn new(mappings_file: PathBuf) -> Self {
        Self { mappings_file }
    }

    pub fn get_token_symbol(&self, faucet_id: &AccountId) -> Result<Option<String>, String> {
        let mappings = self.load_mappings()?;
        Ok(mappings
            .iter()
            .find(|(_, faucet)| faucet.id == faucet_id.to_hex())
            .map(|(symbol, _)| symbol.clone()))
    }

    pub fn get_token_symbol_or_default(&self, faucet_id: &AccountId) -> Result<String, String> {
        self.get_token_symbol(faucet_id)
            .map(|symbol| symbol.unwrap_or("Unknown".to_string()))
    }

    pub fn get_faucet_id(&self, token_symbol: &String) -> Result<Option<AccountId>, String> {
        let mappings = self.load_mappings()?;

        if let Some(faucet_id) = mappings.get(token_symbol).map(|faucet| faucet.id.clone()) {
            Ok(Some(
                AccountId::from_hex(&faucet_id)
                    .map_err(|err| format!("Failed to parse faucet ID: {}", err))?,
            ))
        } else {
            Ok(None)
        }
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
}
