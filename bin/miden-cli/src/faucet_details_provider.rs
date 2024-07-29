use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use miden_client::{
    accounts::AccountId, assets::FungibleAsset, auth::TransactionAuthenticator, crypto::FeltRng,
    rpc::NodeRpcClient, store::Store, Client,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FaucetDetails {
    pub id: String,
    pub decimals: u8,
}
pub struct FaucetDetailsProvider(BTreeMap<String, FaucetDetails>);

impl FaucetDetailsProvider {
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

    /// Parses a fungible Asset and returns it as a tuple of the amount and the faucet account ID hex.
    /// The provided `arg` should be in the format `<AMOUNT>::<ASSET>` where `<AMOUNT>` is the amount
    /// of the asset and `<ASSET>` is either the faucet account ID hex or a symbol tracked by
    /// the token symbol map file. Some examples of valid `arg` values are `100::0x123456789` and
    /// `100::POL`.
    ///
    /// # Errors
    ///
    /// Will return an error if the provided `arg` doesn't match one of the expected formats.
    pub fn parse_fungible_asset<
        N: NodeRpcClient,
        R: FeltRng,
        S: Store,
        A: TransactionAuthenticator,
    >(
        &self,
        client: &Client<N, R, S, A>,
        arg: &str,
    ) -> Result<FungibleAsset, String> {
        let (amount, asset) = arg.split_once("::").ok_or("Separator `::` not found!")?;
        let amount = amount.parse::<f64>().map_err(|err| err.to_string())?;
        let faucet_id = if asset.starts_with("0x") {
            AccountId::from_hex(asset).map_err(|err| err.to_string())?
        } else {
            self.get_faucet_id(&asset.to_string())?
                .ok_or(format!("Token symbol `{asset}` not found in token symbol map file"))?
        };

        let amount = self.faucet_units_from_amount(client, amount, faucet_id)?;

        FungibleAsset::new(faucet_id, amount).map_err(|err| err.to_string())
    }

    pub fn format_fungible_asset<
        N: NodeRpcClient,
        R: FeltRng,
        S: Store,
        A: TransactionAuthenticator,
    >(
        &self,
        client: &Client<N, R, S, A>,
        asset: &FungibleAsset,
    ) -> Result<(String, f64), String> {
        let faucet_id = asset.faucet_id().to_hex();
        let amount = self.amount_from_faucet_units(client, asset.amount(), asset.faucet_id())?;
        Ok((faucet_id, amount))
    }

    // HELPERS
    // ================================================================================================

    fn get_faucet_decimals<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &Client<N, R, S, A>,
        faucet_id: AccountId,
    ) -> Result<u8, String> {
        let (faucet, _) = client
            .get_account(faucet_id)
            .map_err(|err| format!("Error fetching faucet account: {err}"))?;

        let metadata = faucet.storage().get_item(1);
        let decimals: u8 = metadata.as_elements()[1]
            .try_into()
            .map_err(|err| format!("Error parsing faucet metadata: {err}"))?;

        Ok(decimals)
    }

    fn faucet_units_from_amount<
        N: NodeRpcClient,
        R: FeltRng,
        S: Store,
        A: TransactionAuthenticator,
    >(
        &self,
        client: &Client<N, R, S, A>,
        amount: f64,
        faucet_id: AccountId,
    ) -> Result<u64, String> {
        let decimals = self.get_faucet_decimals(client, faucet_id)?;
        let units = amount * 10.0_f64.powi(decimals as i32);

        if units > units.floor() {
            return Err(format!("The amount can't have more than {} decimals", decimals));
        }

        Ok(units as u64)
    }

    fn amount_from_faucet_units<
        N: NodeRpcClient,
        R: FeltRng,
        S: Store,
        A: TransactionAuthenticator,
    >(
        &self,
        client: &Client<N, R, S, A>,
        units: u64,
        faucet_id: AccountId,
    ) -> Result<f64, String> {
        let decimals = self.get_faucet_decimals(client, faucet_id)?;

        let amount = units as f64 / 10.0_f64.powi(decimals as i32);

        Ok(amount)
    }
}
