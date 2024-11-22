use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug, Display, Formatter};

use miden_objects::{
    accounts::{AccountCode, AccountHeader, AccountId, AccountStorageHeader},
    crypto::merkle::MerklePath,
    Digest, Felt,
};
use miden_tx::utils::Deserializable;

use crate::rpc::RpcError;
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

// ACCOUNT HEADER
// ================================================================================================

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

// ACCOUNT PROOF
// ================================================================================================

/// Contains a block number, and a list of account proofs at that block.
pub type AccountProofs = (u32, Vec<AccountProof>);

/// Account state headers.
pub struct StateHeaders {
    pub account_header: AccountHeader,
    pub storage_header: AccountStorageHeader,
    pub code: Option<AccountCode>,
}

/// Represents a proof of existence of an account's state at a specific block number.
pub struct AccountProof {
    /// Account ID.
    account_id: AccountId,
    /// Authentication path from the `account_root` of the block header to the account.
    merkle_proof: MerklePath,
    /// Account hash for the current state.
    account_hash: Digest,
    /// State headers of public accounts.
    state_headers: Option<StateHeaders>,
}

impl AccountProof {
    pub fn new(
        account_id: AccountId,
        merkle_proof: MerklePath,
        account_hash: Digest,
        state_headers: Option<StateHeaders>,
    ) -> Result<Self, AccountProofError> {
        if let Some(StateHeaders { account_header, storage_header: _, code }) = &state_headers {
            if account_header.hash() != account_hash {
                return Err(AccountProofError::InconsistentAccountHash);
            }
            if account_id != account_header.id() {
                return Err(AccountProofError::InconsistentAccountId);
            }
            if let Some(code) = code {
                if code.commitment() != account_header.code_commitment() {
                    return Err(AccountProofError::InconsistentCodeCommitment);
                }
            }
        }

        Ok(Self {
            account_id,
            merkle_proof,
            account_hash,
            state_headers,
        })
    }

    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    pub fn account_header(&self) -> Option<&AccountHeader> {
        self.state_headers.as_ref().map(|headers| &headers.account_header)
    }

    pub fn storage_header(&self) -> Option<&AccountStorageHeader> {
        self.state_headers.as_ref().map(|headers| &headers.storage_header)
    }

    pub fn account_code(&self) -> Option<&AccountCode> {
        if let Some(StateHeaders { code, .. }) = &self.state_headers {
            code.as_ref()
        } else {
            None
        }
    }

    pub fn code_commitment(&self) -> Option<Digest> {
        match &self.state_headers {
            Some(StateHeaders { code: Some(code), .. }) => Some(code.commitment()),
            _ => None,
        }
    }

    pub fn account_hash(&self) -> Digest {
        self.account_hash
    }

    pub fn merkle_proof(&self) -> &MerklePath {
        &self.merkle_proof
    }
}

pub enum AccountProofError {
    InconsistentAccountHash,
    InconsistentAccountId,
    InconsistentCodeCommitment,
}

impl fmt::Display for AccountProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountProofError::InconsistentAccountHash => write!(f,"The received account hash does not match the received account header's account hash"),
            AccountProofError::InconsistentAccountId => write!(f,"The received account ID does not match the received account header's ID"),
            AccountProofError::InconsistentCodeCommitment => write!(f,"The received code commitment does not match the received account header's code commitment"),
        }
    }
}
