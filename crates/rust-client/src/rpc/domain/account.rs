use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug, Display, Formatter};

use miden_objects::{
    Digest, Felt,
    account::{Account, AccountCode, AccountHeader, AccountId, AccountStorageHeader},
    block::{AccountWitness, BlockNumber},
    crypto::merkle::{MerklePath, SmtProof},
};
use miden_tx::utils::{Deserializable, Serializable, ToHex};
use thiserror::Error;

use crate::rpc::{
    RpcError,
    errors::RpcConversionError,
    generated::{
        account::{AccountHeader as ProtoAccountHeader, AccountId as ProtoAccountId},
        requests::get_account_proofs_request,
        responses::{
            AccountStateHeader as ProtoAccountStateHeader, AccountWitness as ProtoAccountWitness,
            StorageSlotMapProof,
        },
    },
};

// ACCOUNT DETAILS
// ================================================================================================

/// Describes the possible responses from the `GetAccountDetails` endpoint for an account.
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

    // Returns the account update summary commitment
    pub fn commitment(&self) -> Digest {
        match self {
            Self::Private(_, summary) | Self::Public(_, summary) => summary.commitment,
        }
    }

    // Returns the associated account if the account is public, otherwise none
    pub fn account(&self) -> Option<&Account> {
        match self {
            Self::Private(..) => None,
            Self::Public(account, _) => Some(account),
        }
    }
}

// ACCOUNT UPDATE SUMMARY
// ================================================================================================

/// Contains public updated information about the account requested.
pub struct AccountUpdateSummary {
    /// Commitment of the account, that represents a commitment to its updated state.
    pub commitment: Digest,
    /// Block number of last account update.
    pub last_block_num: u32,
}

impl AccountUpdateSummary {
    /// Creates a new [`AccountUpdateSummary`].
    pub fn new(commitment: Digest, last_block_num: u32) -> Self {
        Self { commitment, last_block_num }
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
// ================================================================================================

impl From<AccountId> for ProtoAccountId {
    fn from(account_id: AccountId) -> Self {
        Self { id: account_id.to_bytes() }
    }
}

// FROM PROTO ACCOUNT ID
// ================================================================================================

impl TryFrom<ProtoAccountId> for AccountId {
    type Error = RpcConversionError;

    fn try_from(account_id: ProtoAccountId) -> Result<Self, Self::Error> {
        AccountId::read_from_bytes(&account_id.id).map_err(|_| RpcConversionError::NotAValidFelt)
    }
}

// ACCOUNT HEADER
// ================================================================================================

impl ProtoAccountHeader {
    #[allow(dead_code)]
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
// ================================================================================================

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
    #[allow(dead_code)]
    pub fn into_domain(
        self,
        account_id: AccountId,
        known_account_codes: &BTreeMap<Digest, AccountCode>,
    ) -> Result<StateHeaders, RpcError> {
        let ProtoAccountStateHeader {
            header,
            storage_header,
            account_code,
            storage_maps,
        } = self;
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

        // Get map values into slot |-> (key, value, proof) mapping
        let mut storage_slot_proofs: BTreeMap<u8, Vec<SmtProof>> = BTreeMap::new();
        for StorageSlotMapProof { storage_slot, smt_proof } in storage_maps {
            let proof = SmtProof::read_from_bytes(&smt_proof)?;
            match storage_slot_proofs
                .get_mut(&(u8::try_from(storage_slot).expect("there are no more than 256 slots")))
            {
                Some(list) => list.push(proof),
                None => {
                    _ = storage_slot_proofs.insert(
                        u8::try_from(storage_slot).expect("only 256 storage slots"),
                        vec![proof],
                    );
                },
            }
        }

        Ok(StateHeaders {
            account_header,
            storage_header,
            code,
            storage_slots: storage_slot_proofs,
        })
    }
}

// ACCOUNT PROOF
// ================================================================================================

/// Contains a block number, and a list of account proofs at that block.
pub type AccountProofs = (BlockNumber, Vec<AccountProof>);

/// Account state headers.
#[derive(Clone, Debug)]
pub struct StateHeaders {
    // TODO: should this be renamed? or storage_slots moved to AccountProof
    pub account_header: AccountHeader,
    pub storage_header: AccountStorageHeader,
    pub code: AccountCode,
    pub storage_slots: BTreeMap<StorageSlotIndex, Vec<SmtProof>>,
}

/// Represents a proof of existence of an account's state at a specific block number.
#[derive(Clone, Debug)]
pub struct AccountProof {
    /// Account witness.
    account_witness: AccountWitness,
    /// State headers of public accounts.
    state_headers: Option<StateHeaders>,
}

impl AccountProof {
    /// Creates a new [`AccountProof`].
    pub fn new(
        account_witness: AccountWitness,
        state_headers: Option<StateHeaders>,
    ) -> Result<Self, AccountProofError> {
        if let Some(StateHeaders {
            account_header, storage_header: _, code, ..
        }) = &state_headers
        {
            if account_header.commitment() != account_witness.state_commitment() {
                return Err(AccountProofError::InconsistentAccountCommitment);
            }
            if account_header.id() != account_witness.id() {
                return Err(AccountProofError::InconsistentAccountId);
            }
            if code.commitment() != account_header.code_commitment() {
                return Err(AccountProofError::InconsistentCodeCommitment);
            }
        }

        Ok(Self { account_witness, state_headers })
    }

    /// Returns the account ID related to the account proof.
    pub fn account_id(&self) -> AccountId {
        self.account_witness.id()
    }

    /// Returns the account header, if present.
    pub fn account_header(&self) -> Option<&AccountHeader> {
        self.state_headers.as_ref().map(|headers| &headers.account_header)
    }

    /// Returns the storage header, if present.
    pub fn storage_header(&self) -> Option<&AccountStorageHeader> {
        self.state_headers.as_ref().map(|headers| &headers.storage_header)
    }

    /// Returns the account code, if present.
    pub fn account_code(&self) -> Option<&AccountCode> {
        self.state_headers.as_ref().map(|headers| &headers.code)
    }

    /// Returns the code commitment, if account code is present in the state headers.
    pub fn code_commitment(&self) -> Option<Digest> {
        self.account_code().map(AccountCode::commitment)
    }

    /// Returns the current state commitment of the account.
    pub fn account_commitment(&self) -> Digest {
        self.account_witness.state_commitment()
    }

    pub fn account_witness(&self) -> &AccountWitness {
        &self.account_witness
    }

    /// Returns the proof of the account's inclusion.
    pub fn merkle_proof(&self) -> &MerklePath {
        self.account_witness.path()
    }

    /// Deconstructs `AccountProof` into its individual parts.
    pub fn into_parts(self) -> (AccountWitness, Option<StateHeaders>) {
        (self.account_witness, self.state_headers)
    }
}

// ACCOUNT WITNESS
// ================================================================================================

impl TryFrom<ProtoAccountWitness> for AccountWitness {
    type Error = RpcError;

    fn try_from(account_witness: ProtoAccountWitness) -> Result<Self, Self::Error> {
        let state_commitment = account_witness
            .commitment
            .ok_or(RpcError::ExpectedDataMissing(String::from("AccountWitness.StateCommitment")))?
            .try_into()?;
        let merkle_path = account_witness
            .path
            .ok_or(RpcError::ExpectedDataMissing(String::from("AccountWitness.MerklePath")))?
            .try_into()?;
        let account_id = account_witness
            .witness_id
            .ok_or(RpcError::ExpectedDataMissing(String::from("AccountWitness.WitnessId")))?
            .try_into()?;

        let witness = AccountWitness::new(account_id, state_commitment, merkle_path).unwrap();
        Ok(witness)
    }
}

// ACCOUNT STORAGE REQUEST
// ================================================================================================

pub type StorageSlotIndex = u8;
pub type StorageMapKey = Digest;

/// Describes storage slots indices to be requested, as well as a list of keys for each of those
/// slots.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AccountStorageRequirements(BTreeMap<StorageSlotIndex, Vec<StorageMapKey>>);

impl AccountStorageRequirements {
    pub fn new<'a>(
        slots_and_keys: impl IntoIterator<
            Item = (StorageSlotIndex, impl IntoIterator<Item = &'a StorageMapKey>),
        >,
    ) -> Self {
        let map = slots_and_keys
            .into_iter()
            .map(|(slot_index, keys_iter)| {
                let keys_vec: Vec<StorageMapKey> = keys_iter.into_iter().copied().collect();
                (slot_index, keys_vec)
            })
            .collect();

        AccountStorageRequirements(map)
    }

    pub fn inner(&self) -> &BTreeMap<StorageSlotIndex, Vec<StorageMapKey>> {
        &self.0
    }
}

impl From<AccountStorageRequirements> for Vec<get_account_proofs_request::StorageRequest> {
    fn from(value: AccountStorageRequirements) -> Vec<get_account_proofs_request::StorageRequest> {
        let mut requests = Vec::with_capacity(value.0.len());
        for (slot_index, map_keys) in value.0 {
            requests.push(get_account_proofs_request::StorageRequest {
                storage_slot_index: u32::from(slot_index),
                map_keys: map_keys
                    .into_iter()
                    .map(crate::rpc::generated::digest::Digest::from)
                    .collect(),
            });
        }
        requests
    }
}

impl Serializable for AccountStorageRequirements {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        target.write(&self.0);
    }
}

impl Deserializable for AccountStorageRequirements {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        Ok(AccountStorageRequirements(source.read()?))
    }
}

// ERRORS
// ================================================================================================

#[derive(Debug, Error)]
pub enum AccountProofError {
    #[error(
        "the received account commitment doesn't match the received account header's commitment"
    )]
    InconsistentAccountCommitment,
    #[error("the received account id doesn't match the received account header's id")]
    InconsistentAccountId,
    #[error(
        "the received code commitment doesn't match the received account header's code commitment"
    )]
    InconsistentCodeCommitment,
}
