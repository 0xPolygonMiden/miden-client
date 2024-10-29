use alloc::string::{String, ToString};
use core::fmt::{self, Debug, Display, Formatter};

use miden_objects::{
    accounts::{AccountCode, AccountHeader, AccountId, AccountStorageHeader},
    Felt,
};
use miden_tx::utils::Deserializable;

#[cfg(feature = "tonic")]
use crate::rpc::{
    tonic_client::generated::account::AccountHeader as ProtoAccountHeader,
    tonic_client::generated::account::AccountId as ProtoAccountId,
    tonic_client::generated::responses::AccountStateHeader as ProtoAccountStateHeader,
    RpcConversionError,
};
#[cfg(feature = "web-tonic")]
use crate::rpc::{
    web_tonic_client::generated::account::AccountHeader as ProtoAccountHeader,
    web_tonic_client::generated::account::AccountId as ProtoAccountId,
    web_tonic_client::generated::responses::AccountStateHeader as ProtoAccountStateHeader,
    RpcConversionError,
};
use crate::rpc::{RpcError, StateHeaders};

// ACCOUNT ID
// ================================================================================================

impl Display for ProtoAccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("0x{:x}", self.id))
    }
}

impl Debug for ProtoAccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

// INTO PROTO ACCOUNT ID
// ------------------------------------------------------------------------------------------------

impl From<AccountId> for ProtoAccountId {
    fn from(account_id: AccountId) -> Self {
        Self { id: account_id.into() }
    }
}

// FROM PROTO ACCOUNT ID
// ------------------------------------------------------------------------------------------------

impl TryFrom<ProtoAccountId> for AccountId {
    type Error = RpcConversionError;

    fn try_from(account_id: ProtoAccountId) -> Result<Self, Self::Error> {
        account_id.id.try_into().map_err(|_| RpcConversionError::NotAValidFelt)
    }
}

impl ProtoAccountHeader {
    pub fn into_domain(self, account_id: AccountId) -> Result<AccountHeader, RpcError> {
        let ProtoAccountHeader {
            nonce,
            vault_root,
            storage_commitment,
            code_commitment,
        } = self;
        let vault_root = vault_root
            .ok_or(RpcError::ExpectedDataMissing(String::from("AccountHeader.VaultRoot")))?
            .try_into()?;
        let storage_commitment = storage_commitment
            .ok_or(RpcError::ExpectedDataMissing(String::from("AccountHeader.StorageCommitment")))?
            .try_into()?;
        let code_commitment = code_commitment
            .ok_or(RpcError::ExpectedDataMissing(String::from("AccountHeader.CodeCommitment")))?
            .try_into()?;

        Ok(AccountHeader::new(
            account_id,
            Felt::new(nonce),
            vault_root,
            storage_commitment,
            code_commitment,
        ))
    }
}

// FROM PROTO ACCOUNT HEADERS
// ------------------------------------------------------------------------------------------------

impl ProtoAccountStateHeader {
    pub fn into_domain(self, account_id: AccountId) -> Result<StateHeaders, RpcError> {
        let ProtoAccountStateHeader { header, storage_header, account_code } = self;
        let account_header =
            header.ok_or(RpcError::ExpectedDataMissing("Account.StateHeader".to_string()))?;

        let storage_header = AccountStorageHeader::read_from_bytes(&storage_header)?;

        let code = account_code.map(|c| AccountCode::read_from_bytes(&c)).transpose()?;

        Ok(StateHeaders {
            account_header: account_header.into_domain(account_id)?,
            storage_header,
            code,
        })
    }
}
