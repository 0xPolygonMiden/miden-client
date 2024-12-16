//! Contains structures and functions related to FPI (Foreign Procedure Invocation) transactions.
use alloc::string::ToString;
use core::cmp::Ordering;

use miden_objects::{
    accounts::{Account, AccountCode, AccountHeader, AccountId, AccountStorageHeader},
    crypto::merkle::MerklePath,
};
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

use super::TransactionRequestError;
use crate::rpc::domain::accounts::{AccountProof, StateHeaders};

// FOREIGN ACCOUNT
// ================================================================================================

/// Account types for foreign procedure invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForeignAccount {
    /// Public account data will be retrieved from the network at execution time, based on the
    /// account id.
    Public(AccountId),
    /// Private account data requires [ForeignAccountData] to be input. The proof of the account's
    /// existence will be retrieved from the network at execution time.
    Private(ForeignAccountInputs),
}

impl ForeignAccount {
    pub fn public(account_id: AccountId) -> Result<Self, TransactionRequestError> {
        if !account_id.is_public() {
            return Err(TransactionRequestError::InvalidForeignAccountId(account_id));
        }

        Ok(Self::Public(account_id))
    }

    pub fn private(
        account: impl Into<ForeignAccountInputs>,
    ) -> Result<Self, TransactionRequestError> {
        let foreign_account: ForeignAccountInputs = account.into();
        if foreign_account.account_header().id().is_public() {
            return Err(TransactionRequestError::InvalidForeignAccountId(
                foreign_account.account_header().id(),
            ));
        }

        Ok(Self::Private(foreign_account))
    }

    pub fn account_id(&self) -> AccountId {
        match self {
            ForeignAccount::Public(account_id) => *account_id,
            ForeignAccount::Private(foreign_account_inputs) => {
                foreign_account_inputs.account_header.id()
            },
        }
    }

    pub fn account_code(self) -> Option<AccountCode> {
        match self {
            ForeignAccount::Public(_) => None,
            ForeignAccount::Private(foreign_account_inputs) => {
                Some(foreign_account_inputs.account_code)
            },
        }
    }
}

impl Ord for ForeignAccount {
    fn cmp(&self, other: &Self) -> Ordering {
        self.account_id().cmp(&other.account_id())
    }
}

impl PartialOrd for ForeignAccount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serializable for ForeignAccount {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        match self {
            ForeignAccount::Public(account_id) => {
                target.write(0u8);
                account_id.write_into(target);
            },
            ForeignAccount::Private(foreign_account_inputs) => {
                target.write(1u8);
                foreign_account_inputs.write_into(target);
            },
        }
    }
}

impl Deserializable for ForeignAccount {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let account_type: u8 = source.read_u8()?;
        match account_type {
            0 => {
                let account_id = AccountId::read_from(source)?;
                Ok(ForeignAccount::Public(account_id))
            },
            1 => {
                let foreign_inputs = ForeignAccountInputs::read_from(source)?;
                Ok(ForeignAccount::Private(foreign_inputs))
            },
            _ => Err(DeserializationError::InvalidValue("Invalid account type".to_string())),
        }
    }
}

// FOREIGN ACCOUNT INPUTS
// ================================================================================================

/// Contains information about a foreign account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignAccountInputs {
    /// Account header of the foreign account.
    account_header: AccountHeader,
    /// Header information about the account's storage.
    storage_header: AccountStorageHeader,
    /// Code associated with the account.
    account_code: AccountCode,
}

impl ForeignAccountInputs {
    /// Creates a new [ForeignAccountData]
    pub fn new(
        account_header: AccountHeader,
        storage_header: AccountStorageHeader,
        account_code: AccountCode,
    ) -> ForeignAccountInputs {
        ForeignAccountInputs {
            account_header,
            storage_header,
            account_code,
        }
    }

    /// Returns the account's [AccountHeader].
    pub fn account_header(&self) -> &AccountHeader {
        &self.account_header
    }

    /// Returns the account's [AccountStorageHeader].
    pub fn storage_header(&self) -> &AccountStorageHeader {
        &self.storage_header
    }

    /// Returns the account's [AccountCode].
    pub fn account_code(&self) -> &AccountCode {
        &self.account_code
    }

    /// Consumes the [ForeignAccountData] and returns its parts.
    pub fn into_parts(self) -> (AccountHeader, AccountStorageHeader, AccountCode) {
        (self.account_header, self.storage_header, self.account_code)
    }
}

impl From<Account> for ForeignAccountInputs {
    fn from(value: Account) -> Self {
        let account_code: AccountCode = value.code().clone();
        let storage_header: AccountStorageHeader = value.storage().get_header();
        let account_header: AccountHeader = value.into();

        ForeignAccountInputs::new(account_header, storage_header, account_code)
    }
}

impl Serializable for ForeignAccountInputs {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.account_header.write_into(target);
        self.storage_header.write_into(target);
        self.account_code.write_into(target);
    }
}

impl Deserializable for ForeignAccountInputs {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let account_header = AccountHeader::read_from(source)?;
        let storage_header = AccountStorageHeader::read_from(source)?;
        let account_code = AccountCode::read_from(source)?;
        Ok(ForeignAccountInputs::new(account_header, storage_header, account_code))
    }
}

impl TryFrom<AccountProof> for (ForeignAccountInputs, MerklePath) {
    type Error = TransactionRequestError;

    fn try_from(value: AccountProof) -> Result<Self, Self::Error> {
        let (_, merkle_proof, _, state_headers) = value.into_parts();
        if let Some(StateHeaders { account_header, storage_header, code }) = state_headers {
            let inputs = ForeignAccountInputs::new(account_header, storage_header, code);
            return Ok((inputs, merkle_proof));
        }
        Err(TransactionRequestError::ForeignAccountDataMissing)
    }
}
