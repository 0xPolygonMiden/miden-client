use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use miden_client::accounts::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FaucetDetails {
    pub id: String,
    pub decimals: u8,
}
pub struct TokenSymbolMap(BTreeMap<String, FaucetDetails>);

impl TokenSymbolMap {
    pub fn new(mappings_file: PathBuf) -> Result<Self, String> {
        let mappings: BTreeMap<String, FaucetDetails> = match std::fs::read_to_string(mappings_file)
        {
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

        let mut faucet_ids = BTreeSet::new();
        for faucet in mappings.values() {
            if !faucet_ids.insert(faucet.id.clone()) {
                return Err(format!("Faucet ID '{}' appears more than once", faucet.id));
            }
        }

        Ok(Self(mappings))
    }

    pub fn get_token_symbol(&self, faucet_id: &AccountId) -> Option<String> {
        self.0
            .iter()
            .find(|(_, faucet)| faucet.id == faucet_id.to_hex())
            .map(|(symbol, _)| symbol.clone())
    }

    pub fn get_token_symbol_or_default(&self, faucet_id: &AccountId) -> String {
        self.get_token_symbol(faucet_id).unwrap_or("Unknown".to_string())
    }

    pub fn get_faucet_id(&self, token_symbol: &String) -> Result<Option<AccountId>, String> {
        if let Some(faucet_id) = self.0.get(token_symbol).map(|faucet| faucet.id.clone()) {
            Ok(Some(
                AccountId::from_hex(&faucet_id)
                    .map_err(|err| format!("Failed to parse faucet ID: {}", err))?,
            ))
        } else {
            Ok(None)
        }
    }
}
