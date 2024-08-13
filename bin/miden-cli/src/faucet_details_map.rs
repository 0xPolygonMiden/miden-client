use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use miden_client::{accounts::AccountId, assets::FungibleAsset};
use serde::{Deserialize, Serialize};

/// Stores the detail information of a faucet to be stored in the token symbol map file.
#[derive(Debug, Serialize, Deserialize)]
pub struct FaucetDetails {
    pub id: String,
    pub decimals: u8,
}
pub struct FaucetDetailsMap(BTreeMap<String, FaucetDetails>);

impl FaucetDetailsMap {
    /// Creates a new instance of the `FaucetDetailsMap` struct by loading the token symbol map file
    /// from the specified `token_symbol_map_filepath`. If the file doesn't exist, an empty map is
    /// created.
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
    /// Some examples of valid `arg` values are `100::0xabcdef0123456789` and `1.23::POL`.
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
            let FaucetDetails { id, decimals: faucet_decimals } = self
                .get_faucet_details(&asset.to_string())
                .ok_or(format!("Token symbol `{asset}` not found in token symbol map file"))?;

            // Validate that the amount is a valid number.
            amount.parse::<f64>().map_err(|err| err.to_string())?;

            // Convert from decimal to integer.
            let amount = decimal_to_integer(amount, decimals)?;

            let faucet_id = AccountId::from_hex(id).map_err(|err| err.to_string())?;

            (faucet_id, amount)
        };

        FungibleAsset::new(faucet_id, amount).map_err(|err| err.to_string())
    }

    /// Formats a [FungibleAsset] into a tuple containing the faucet and the amount. The returned values
    /// depend on whether the faucet is tracked by the token symbol map file or not:
    /// - If the faucet is tracked, the token symbol is returned along with the amount in the token's
    ///   decimals.
    /// - If the faucet is not tracked, the faucet ID is returned along with the amount in base units.
    pub fn format_fungible_asset(&self, asset: &FungibleAsset) -> (String, String) {
        if let Some(token_symbol) = self.get_token_symbol(&asset.faucet_id()) {
            let decimals = self
                .get_faucet_details(&token_symbol)
                .expect("Token symbol should be present in the token symbol map")
                .decimals;
            let amount = format_amount_from_faucet_units(asset.amount(), decimals);

            (token_symbol, amount)
        } else {
            (asset.faucet_id().to_hex(), asset.amount().to_string())
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
}

/// Converts an amount in the faucet base units to the token's decimals. This amount should only
/// be used for display purposes and should not be used for calculations as it may lose precision.
fn format_amount_from_faucet_units(units: u64, decimals: u8) -> String {
    let units_str = units.to_string();
    let len = units_str.len();

    if decimals as usize >= len {
        // Handle cases where the number of decimals is greater than the length of units
        let leading_zeros = "0.".to_owned() + &"0".repeat(decimals as usize - len) + &units_str;
        leading_zeros
    } else {
        // Insert the decimal point at the correct position
        let integer_part = &units_str[..len - decimals as usize];
        let fractional_part = &units_str[len - decimals as usize..];
        format!("{}.{}", integer_part, fractional_part)
    }
}

/// Converts a decimal number, represented as a string, into an integer by shifting
/// the decimal point to the right by a specified number of decimal places.
fn decimal_to_integer(decimal_str: &str, n_decimals: &u8) -> Result<u64, String> {
    // Split the string on the decimal point
    let parts: Vec<&str> = decimal_str.split('.').collect();

    // Get the integer part
    let integer_part = parts[0];

    // Get the fractional part and pad it if necessary
    let mut fractional_part = if parts.len() > 1 {
        parts[1].to_string()
    } else {
        String::new()
    };

    // Add extra zeros if the fractional part is shorter than N decimals
    while fractional_part.len() < n_decimals {
        fractional_part.push('0');
    }

    // Combine the integer and padded fractional part
    let combined = format!("{}{}", integer_part, &fractional_part[0..n_decimals]);

    // Convert the combined string to an integer
    combined.parse::<u64>()
}
