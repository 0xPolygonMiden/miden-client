use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug, Display, Formatter};

use miden_objects::{
    accounts::{Account, AccountCode, AccountHeader, AccountId, AccountStorageHeader},
    crypto::merkle::MerklePath,
    Digest, Felt,
};
use miden_tx::utils::{Deserializable, Serializable, ToHex};
use thiserror::Error;

use crate::rpc::{
    errors::RpcConversionError,
    generated::{
        account::{AccountHeader as ProtoAccountHeader, AccountId as ProtoAccountId},
        responses::AccountStateHeader as ProtoAccountStateHeader,
    },
    RpcError,
};

// ACCOUNT DETAILS
// ================================================================================================

/// Describes the possible responses from the `GetAccountDetails` endpoint for an account
pub enum AccountDetails {
    /// Private accounts are stored off-chain. Only a commitment to the state of the account is
    /// shared with the network. The full account state is to be tracked locally.
    Private(AccountId, AccountUpdateSummary),
    /// Public accounts are recorded on-chain. As such, its state is shared with the network and
    /// can always be retrieved through the appropriate RPC method.
    Public(Account, AccountUpdateSummary),
}

impl AccountDetails {
    /// Returns the account ID.
    pub fn account_id(&self) -> AccountId {
        match self {
            Self::Private(account_id, _) => *account_id,
            Self::Public(account, _) => account.id(),
        }
    }

    pub fn hash(&self) -> Digest {
        match self {
            Self::Private(_, summary) | Self::Public(_, summary) => summary.hash,
        }
    }
}

// ACCOUNT UPDATE SUMMARY
// ================================================================================================

/// Contains public updated information about the account requested.
pub struct AccountUpdateSummary {
    /// Hash of the account, that represents a commitment to its updated state.
    pub hash: Digest,
    /// Block number of last account update.
    pub last_block_num: u32,
}

impl AccountUpdateSummary {
    /// Creates a new [AccountUpdateSummary].
    pub fn new(hash: Digest, last_block_num: u32) -> Self {
        Self { hash, last_block_num }
    }
}

// ACCOUNT ID
// ================================================================================================

impl Display for ProtoAccountId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("0x{}", self.id.to_hex()))
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
        Self { id: account_id.to_bytes() }
    }
}

// FROM PROTO ACCOUNT ID
// ------------------------------------------------------------------------------------------------

impl TryFrom<ProtoAccountId> for AccountId {
    type Error = RpcConversionError;

    fn try_from(account_id: ProtoAccountId) -> Result<Self, Self::Error> {
        AccountId::read_from_bytes(&account_id.id).map_err(|_| RpcConversionError::NotAValidFelt)
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
    /// Converts the RPC response into `StateHeaders`.
    ///
    /// The RPC response may omit unchanged account codes. If so, this function uses
    /// `known_account_codes` to fill in the missing code. If a required code cannot be found in
    /// the response or `known_account_codes`, an error is returned.
    ///
    /// # Errors
    /// - If account code is missing both on `self` and `known_account_codes`
    /// - If data cannot be correctly deserialized
    pub fn into_domain(
        self,
        account_id: AccountId,
        known_account_codes: &BTreeMap<Digest, AccountCode>,
    ) -> Result<StateHeaders, RpcError> {
        let ProtoAccountStateHeader { header, storage_header, account_code } = self;
        let account_header = header
            .ok_or(RpcError::ExpectedDataMissing("Account.StateHeader".to_string()))?
            .into_domain(account_id)?;

        let storage_header = AccountStorageHeader::read_from_bytes(&storage_header)?;

        // If an account code was received, it means the previously known account code is no longer
        // valid. If it was not, it means we sent a code commitment that matched and so our code
        // is still valid
        let code = {
            let received_code =
                account_code.map(|c| AccountCode::read_from_bytes(&c)).transpose()?;
            match received_code {
                Some(code) => code,
                None => known_account_codes
                    .get(&account_header.code_commitment())
                    .ok_or(RpcError::InvalidResponse(
                        "Account code was not provided, but the response did not contain it either"
                            .to_string(),
                    ))?
                    .clone(),
            }
        };

        Ok(StateHeaders { account_header, storage_header, code })
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
    pub code: AccountCode,
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
            if code.commitment() != account_header.code_commitment() {
                return Err(AccountProofError::InconsistentCodeCommitment);
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
        self.state_headers.as_ref().map(|headers| &headers.code)
    }

    pub fn state_headers(&self) -> Option<&StateHeaders> {
        self.state_headers.as_ref()
    }

    pub fn code_commitment(&self) -> Option<Digest> {
        self.account_code().map(|c| c.commitment())
    }

    pub fn account_hash(&self) -> Digest {
        self.account_hash
    }

    pub fn merkle_proof(&self) -> &MerklePath {
        &self.merkle_proof
    }

    /// Deconstructs `AccountProof` into its individual parts.
    pub fn into_parts(self) -> (AccountId, MerklePath, Digest, Option<StateHeaders>) {
        (self.account_id, self.merkle_proof, self.account_hash, self.state_headers)
    }
}

// ERRORS
// ================================================================================================

#[derive(Debug, Error)]
pub enum AccountProofError {
    #[error("the received account hash doesn't match the received account header's hash")]
    InconsistentAccountHash,
    #[error("the received account id doesn't match the received account header's id")]
    InconsistentAccountId,
    #[error(
        "the received code commitment doesn't match the received account header's code commitment"
    )]
    InconsistentCodeCommitment,
}
