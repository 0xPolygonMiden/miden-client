//! Contains structures and functions related to FPI (Foreign Procedure Invocation) transactions.
use alloc::{string::ToString, vec::Vec};
use core::cmp::Ordering;

use miden_objects::{
    account::{Account, AccountCode, AccountHeader, AccountId, AccountStorageHeader, StorageSlot},
    block::AccountWitness,
    crypto::merkle::SmtProof,
    transaction::ForeignAccountInputs,
};
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

use super::TransactionRequestError;
use crate::rpc::domain::account::{AccountProof, AccountStorageRequirements, StateHeaders};

// FOREIGN ACCOUNT
// ================================================================================================

/// Account types for foreign procedure invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum ForeignAccount {
    /// Public account data will be retrieved from the network at execution time, based on the
    /// account ID. The second element of the tuple indicates which storage slot indices
    /// and map keys are desired to be retrieved.
    Public(AccountId, AccountStorageRequirements),
    /// Private account data requires [`ForeignAccountInformation`] to be passed. An account witness
    /// will be retrieved from the network at execution time so that it can be used as inputs to
    /// the transaction kernel.
    Private(ForeignAccountInformation),
}

impl ForeignAccount {
    /// Creates a new [`ForeignAccount::Public`]. The account's components (code, storage header and
    /// inclusion proof) will be retrieved at execution time, alongside particular storage slot
    /// maps correspondent to keys passed in `indices`.
    pub fn public(
        account_id: AccountId,
        storage_requirements: AccountStorageRequirements,
    ) -> Result<Self, TransactionRequestError> {
        if !account_id.is_public() {
            return Err(TransactionRequestError::InvalidForeignAccountId(account_id));
        }

        Ok(Self::Public(account_id, storage_requirements))
    }

    /// Creates a new [`ForeignAccount::Private`]. A proof of the account's inclusion will be
    /// retrieved at execution time.
    pub fn private(
        account: impl Into<ForeignAccountInformation>,
    ) -> Result<Self, TransactionRequestError> {
        let foreign_account: ForeignAccountInformation = account.into();
        if foreign_account.account_header().id().is_public() {
            return Err(TransactionRequestError::InvalidForeignAccountId(
                foreign_account.account_header().id(),
            ));
        }

        Ok(Self::Private(foreign_account))
    }

    pub fn storage_slot_requirements(&self) -> AccountStorageRequirements {
        match self {
            ForeignAccount::Public(_, account_storage_requirements) => {
                account_storage_requirements.clone()
            },
            ForeignAccount::Private(_) => AccountStorageRequirements::default(),
        }
    }

    /// Returns the foreign account's [`AccountId`].
    pub fn account_id(&self) -> AccountId {
        match self {
            ForeignAccount::Public(account_id, _) => *account_id,
            ForeignAccount::Private(foreign_account_inputs) => {
                foreign_account_inputs.account_header.id()
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
            ForeignAccount::Public(account_id, storage_requirements) => {
                target.write(0u8);
                account_id.write_into(target);
                storage_requirements.write_into(target);
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
                let storage_requirements = AccountStorageRequirements::read_from(source)?;
                Ok(ForeignAccount::Public(account_id, storage_requirements))
            },
            1 => {
                let foreign_inputs = ForeignAccountInformation::read_from(source)?;
                Ok(ForeignAccount::Private(foreign_inputs))
            },
            _ => Err(DeserializationError::InvalidValue("Invalid account type".to_string())),
        }
    }
}

// FOREIGN ACCOUNT INFORMATION
// ================================================================================================

/// Contains information about a foreign account, with everything required to execute its code from
/// the context of the native account.
///
/// At the moment of transaction execution ([`crate::Client::new_transaction()`]), an account
/// witness is fetched for the account and this struct can be converted into
/// [`ForeignAccountInputs`], which is then used as inputs to the transaction kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignAccountInformation {
    /// Account header of the foreign account.
    account_header: AccountHeader,
    /// Header information about the account's storage.
    storage_header: AccountStorageHeader,
    /// Code associated with the account.
    account_code: AccountCode,
    /// Storage SMT proof for storage map values that the transaction will access.
    storage_map_proofs: Vec<SmtProof>,
}

impl ForeignAccountInformation {
    /// Creates a new [`ForeignAccountInformation`]
    pub fn new(
        account_header: AccountHeader,
        storage_header: AccountStorageHeader,
        account_code: AccountCode,
        storage_map_proofs: Vec<SmtProof>,
    ) -> ForeignAccountInformation {
        ForeignAccountInformation {
            account_header,
            storage_header,
            account_code,
            storage_map_proofs,
        }
    }

    /// Creates a new [`ForeignAccountInputs`] from an [`Account`] and a list of storage keys.
    ///
    /// # Errors
    ///
    /// - If one of the specified slots in `storage_requirements` is not a map-type slot or it is
    ///   not found.
    pub fn from_account(
        account: Account,
        storage_requirements: &AccountStorageRequirements,
    ) -> Result<ForeignAccountInformation, TransactionRequestError> {
        // Get required proofs
        let mut smt_proofs = vec![];
        for (slot_index, keys) in storage_requirements.inner() {
            for key in keys {
                let slot = account.storage().slots().get(*slot_index as usize);
                match slot {
                    Some(StorageSlot::Map(map)) => {
                        smt_proofs.push(map.open(key));
                    },
                    Some(StorageSlot::Value(_)) => {
                        return Err(
                            TransactionRequestError::ForeignAccountStorageSlotInvalidIndex(
                                *slot_index,
                            ),
                        );
                    },
                    None => {
                        return Err(TransactionRequestError::StorageSlotNotFound(
                            *slot_index,
                            account.id(),
                        ));
                    },
                }
            }
        }

        let account_code: AccountCode = account.code().clone();
        let storage_header: AccountStorageHeader = account.storage().get_header();
        let account_header: AccountHeader = account.into();

        Ok(ForeignAccountInformation::new(
            account_header,
            storage_header,
            account_code,
            smt_proofs,
        ))
    }

    /// Returns the account's [`AccountHeader`]
    pub fn account_header(&self) -> &AccountHeader {
        &self.account_header
    }

    /// Returns the account's [`AccountStorageHeader`].
    pub fn storage_header(&self) -> &AccountStorageHeader {
        &self.storage_header
    }

    /// Returns the account's storage maps.
    pub fn storage_map_proofs(&self) -> &[SmtProof] {
        &self.storage_map_proofs
    }

    /// Returns the account's [`AccountCode`].
    pub fn account_code(&self) -> &AccountCode {
        &self.account_code
    }

    /// Extends the storage proofs with the input `smt_proofs` and returns the new structure
    #[must_use]
    pub fn with_storage_map_proofs(
        mut self,
        smt_proofs: impl IntoIterator<Item = SmtProof>,
    ) -> Self {
        self.storage_map_proofs.extend(smt_proofs);
        self
    }

    /// Consumes the [`ForeignAccountInformation`] and, with the `merkle_path`, instantiates a
    /// [`ForeignAccountInputs`]. The merkle path should be a valid proof of inclusion of the
    /// account at a specific block in the MMR.
    ///
    /// The [`ForeignAccountInputs`] can then be passed as transaction arguments for transaction
    /// execution.
    pub fn into_foreign_account_inputs(
        self,
        account_witness: AccountWitness,
    ) -> ForeignAccountInputs {
        let (header, storage_header, account_code, storage_map_proofs) = self.into_parts();
        ForeignAccountInputs::new(
            header,
            storage_header,
            account_code,
            account_witness,
            storage_map_proofs,
        )
    }

    /// Consumes the [`ForeignAccountInputs`] and returns its parts.
    pub fn into_parts(self) -> (AccountHeader, AccountStorageHeader, AccountCode, Vec<SmtProof>) {
        (
            self.account_header,
            self.storage_header,
            self.account_code,
            self.storage_map_proofs,
        )
    }
}

impl Serializable for ForeignAccountInformation {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        self.account_header.write_into(target);
        self.storage_header.write_into(target);
        self.account_code.write_into(target);
        self.storage_map_proofs.write_into(target);
    }
}

impl Deserializable for ForeignAccountInformation {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let account_header = AccountHeader::read_from(source)?;
        let storage_header = AccountStorageHeader::read_from(source)?;
        let account_code = AccountCode::read_from(source)?;
        let storage_maps = Vec::<SmtProof>::read_from(source)?;
        Ok(ForeignAccountInformation::new(
            account_header,
            storage_header,
            account_code,
            storage_maps,
        ))
    }
}

impl TryFrom<AccountProof> for ForeignAccountInputs {
    type Error = TransactionRequestError;

    fn try_from(value: AccountProof) -> Result<Self, Self::Error> {
        let (witness, state_headers) = value.into_parts();

        if let Some(StateHeaders {
            account_header,
            storage_header,
            code,
            storage_slots,
        }) = state_headers
        {
            // discard slot indices - not needed for execution
            let storage_map_proofs =
                storage_slots.into_iter().flat_map(|(_, slots)| slots).collect();

            return Ok(ForeignAccountInputs::new(
                account_header,
                storage_header,
                code,
                witness,
                storage_map_proofs,
            ));
        }
        Err(TransactionRequestError::ForeignAccountDataMissing)
    }
}
