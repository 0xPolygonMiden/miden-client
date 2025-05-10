use miden_objects::account::AccountType as NativeAccountType;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub enum AccountType {
    FungibleFaucet,
    NonFungibleFaucet,
    RegularAccountImmutableCode,
    RegularAccountUpdatableCode,
}

// CONVERSIONS
// ================================================================================================

impl From<AccountType> for NativeAccountType {
    fn from(value: AccountType) -> Self {
        match value {
            AccountType::FungibleFaucet => NativeAccountType::FungibleFaucet,
            AccountType::NonFungibleFaucet => NativeAccountType::NonFungibleFaucet,
            AccountType::RegularAccountImmutableCode => {
                NativeAccountType::RegularAccountImmutableCode
            },
            AccountType::RegularAccountUpdatableCode => {
                NativeAccountType::RegularAccountUpdatableCode
            },
        }
    }
}

impl From<&AccountType> for NativeAccountType {
    fn from(value: &AccountType) -> Self {
        match value {
            AccountType::FungibleFaucet => NativeAccountType::FungibleFaucet,
            AccountType::NonFungibleFaucet => NativeAccountType::NonFungibleFaucet,
            AccountType::RegularAccountImmutableCode => {
                NativeAccountType::RegularAccountImmutableCode
            },
            AccountType::RegularAccountUpdatableCode => {
                NativeAccountType::RegularAccountUpdatableCode
            },
        }
    }
}
