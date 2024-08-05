use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use miden_client::{accounts::AccountId, assets::FungibleAsset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FaucetDetails {
    pub id: String,
    pub decimals: u8,
}
pub struct FaucetDetailsMap(BTreeMap<String, FaucetDetails>);

impl FaucetDetailsMap {
    pub fn new(token_symbol_map_filepath: PathBuf) -> Result<Self, String> {
        let token_symbol_map: BTreeMap<String, FaucetDetails> =
            match std::fs::read_to_string(token_symbol_map_filepath) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(token_symbol_map) => token_symbol_map,
                    Err(err) => {
                        return Err(format!("Failed to parse token_symbol_map file: {}", err))
                    },
                },
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        return Err(format!("Failed to read token_symbol_map file: {}", err));
                    }
                    BTreeMap::new()
                },
            };

        let mut faucet_ids = BTreeSet::new();
        for faucet in token_symbol_map.values() {
            if !faucet_ids.insert(faucet.id.clone()) {
                return Err(format!(
                    "Faucet ID '{}' appears more than once in the token symbol map",
                    faucet.id
                ));
            }
        }

        Ok(Self(token_symbol_map))
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

    /// Parses a string representing a [FungibleAsset]. There are two accepted formats for the string:
    /// - `<AMOUNT>::<FAUCET_ID>` where `<AMOUNT>` is in the faucet base units.
    /// - `<AMOUNT>::<TOKEN_SYMBOL>` where `<AMOUNT>` is a decimal number representing the quantity of 
    ///   the token (specified to the precision allowed by the token's decimals), and `<TOKEN_SYMBOL>`
    ///   is a symbol tracked in the token symbol map file.
    ///
    /// Some examples of valid `arg` values are `100::0x123456789` and `1.23::POL`.
    ///
    /// # Errors
    ///
    /// Will return an error if:
    /// - The provided `arg` doesn't match one of the expected formats.
    /// - A faucet ID was provided but the amount is not in base units.
    /// - The amount has more than the allowed number of decimals.
    /// - The token symbol is not present in the token symbol map file.
    pub fn parse_fungible_asset(&self, arg: &str) -> Result<FungibleAsset, String> {
        let (amount, asset) = arg.split_once("::").ok_or("Separator `::` not found!")?;
        let (faucet_id, amount) = if asset.starts_with("0x") {
            let amount = amount.parse::<u64>().map_err(|err| err.to_string())?;
            (AccountId::from_hex(asset).map_err(|err| err.to_string())?, amount)
        } else {
            let float_amount = amount.parse::<f64>().map_err(|err| err.to_string())?;
            let amount_decimals = amount.split('.').last().unwrap_or("").len();

            let amount = float_amount * 10.0_f64.powi(amount_decimals as i32);
            assert!(amount.fract() == 0.0, "Amount should be an integer");

            let FaucetDetails { id, decimals: faucet_decimals } = self
                .get_faucet_details(&asset.to_string())
                .ok_or(format!("Token symbol `{asset}` not found in token symbol map file"))?;

            let faucet_id = AccountId::from_hex(id).map_err(|err| err.to_string())?;
            let amount = self.faucet_units_from_amount(
                amount as u64,
                amount_decimals as u8,
                *faucet_decimals,
            )?;

            (faucet_id, amount)
        };

        FungibleAsset::new(faucet_id, amount).map_err(|err| err.to_string())
    }

    /// Formats a [FungibleAsset] into a tuple containing the faucet and the amount. The returned values
    /// depend on whether the faucet is tracked by the token symbol map file or not:
    /// - If the faucet is tracked, the token symbol is returned along with the amount in the token's
    ///   decimals.
    /// - If the faucet is not tracked, the faucet ID is returned along with the amount in base units.
    pub fn format_fungible_asset(&self, asset: &FungibleAsset) -> (String, f64) {
        if let Some(token_symbol) = self.get_token_symbol(&asset.faucet_id()) {
            let decimals = self
                .get_faucet_details(&token_symbol)
                .expect("Token symbol should be present in the token symbol map")
                .decimals;
            let amount = self.format_amount_from_faucet_units(asset.amount(), decimals);

            (token_symbol, amount)
        } else {
            (asset.faucet_id().to_hex(), asset.amount() as f64)
        }
    }

    // HELPERS
    // ================================================================================================

    fn get_faucet_details(&self, token_symbol: &String) -> Option<&FaucetDetails> {
        Some(
            self.0
                .get(token_symbol)
                .expect("Token symbol should be present in the token symbol map"),
        )
    }

    /// Converts an amount in the token's decimals to the faucet base units. `amount` should be
    /// the decimal value converted to an integer (e.g., 1.23 -> 123) and `amount_decimals` should
    /// be the number of decimals in the amount.
    fn faucet_units_from_amount(
        &self,
        amount: u64,
        amount_decimals: u8,
        faucet_decimals: u8,
    ) -> Result<u64, String> {
        if amount_decimals > faucet_decimals {
            return Err(format!("The amount can't have more than {} decimals", faucet_decimals));
        }

        let units = amount * 10_i32.pow((faucet_decimals - amount_decimals).into()) as u64;

        Ok(units)
    }

    /// Converts an amount in the faucet base units to the token's decimals. This amount should only
    /// be used for display purposes and should not be used for calculations as it may lose precision.
    fn format_amount_from_faucet_units(&self, units: u64, decimals: u8) -> f64 {
        units as f64 / 10.0_f64.powi(decimals as i32)
    }
}
