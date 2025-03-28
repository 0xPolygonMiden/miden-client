//! Contains structures and functions related to transaction creation.

use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};

use miden_lib::account::interface::{AccountInterface, AccountInterfaceError};
use miden_objects::{
    Digest, Felt, NoteError, Word,
    account::AccountId,
    assembly::AssemblyError,
    asset::Asset,
    crypto::merkle::MerkleStore,
    note::{Note, NoteDetails, NoteId, NoteRecipient, NoteTag, NoteType, PartialNote},
    transaction::{TransactionArgs, TransactionScript},
    vm::AdviceMap,
};
use miden_tx::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};
use thiserror::Error;

use crate::transaction::TransactionScriptTemplate::SendNotes;

mod builder;
pub use builder::{PaymentNoteDescription, SwapNoteDescription, TransactionRequestBuilder};

mod foreign;
pub use foreign::{ForeignAccount, ForeignAccountInputs};

// TRANSACTION REQUEST
// ================================================================================================

pub type NoteArgs = Word;

/// Specifies a transaction script to be executed in a transaction.
///
/// A transaction script is a program which is executed after scripts of all input notes have been
/// executed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransactionScriptTemplate {
    /// Specifies the exact transaction script to be executed in a transaction.
    CustomScript(TransactionScript),
    /// Specifies that the transaction script must create the specified output notes.
    ///
    /// It is up to the client to determine how the output notes will be created and this will
    /// depend on the capabilities of the account the transaction request will be applied to.
    /// For example, for Basic Wallets, this may involve invoking `create_note` procedure.
    SendNotes(Vec<OwnNoteTemplate>),
}

/// Describes the type of own note that should be generated as part of a [`TransactionRequest`].
///
/// On execution, a transaction script is generated to output the desired notes, based on data
/// such as the sender account ID and its account interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnNoteTemplate {
    /// Defines a sender-agnostic P2ID note.
    P2ID(PaymentNoteDescription, NoteType),
    /// Defines a sender-agnostic swap note.
    Swap(SwapNoteDescription, NoteType),
    /// Defines a specific note.
    Note(Note),
}

impl OwnNoteTemplate {
    /// Gets the outgoing assets from the generated note.
    pub fn outgoing_assets(&self) -> Vec<Asset> {
        match self {
            OwnNoteTemplate::P2ID(payment_transaction_data, _) => {
                payment_transaction_data.assets().clone()
            },
            OwnNoteTemplate::Swap(swap_transaction_data, _) => {
                vec![swap_transaction_data.offered_asset()]
            },
            OwnNoteTemplate::Note(partial_note) => partial_note.assets().iter().copied().collect(),
        }
    }

    /// Creates the output [`Note`] object based on the sender ID.
    pub fn get_outgoing_note(&self, sender_account_id: AccountId) -> Note {
        match self {
            OwnNoteTemplate::P2ID(payment_transaction_data, note_type) => {
                payment_transaction_data.get_note(sender_account_id, *note_type)
            },
            OwnNoteTemplate::Swap(swap_transaction_data, note_type) => {
                swap_transaction_data.get_note(sender_account_id, *note_type).0
            },
            OwnNoteTemplate::Note(note) => note.clone(),
        }
    }

    /// Gets the note details that will be created as part of future transactions.
    ///
    /// These future notes are descriptions of notes that this note script output.
    pub fn get_future_notes(
        &self,
        sender_account_id: AccountId,
    ) -> Result<Option<(NoteDetails, NoteTag)>, NoteError> {
        match self {
            OwnNoteTemplate::Swap(swap_transaction_data, note_type) => {
                let (_, note_details, note_tag) =
                    swap_transaction_data.get_note(sender_account_id, *note_type);
                Ok(Some((note_details, note_tag)))
            },
            OwnNoteTemplate::P2ID(..) | OwnNoteTemplate::Note(..) => Ok(None),
        }
    }
}

impl Serializable for OwnNoteTemplate {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            OwnNoteTemplate::P2ID(payment_data, note_type) => {
                target.write_u8(0);
                payment_data.write_into(target);
                note_type.write_into(target);
            },
            OwnNoteTemplate::Swap(swap_data, note_type) => {
                target.write_u8(1);
                swap_data.write_into(target);
                note_type.write_into(target);
            },
            OwnNoteTemplate::Note(note) => {
                target.write_u8(2);
                target.write(note);
            },
        }
    }
}

impl Deserializable for OwnNoteTemplate {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        match source.read_u8()? {
            0 => {
                let payment_data = PaymentNoteDescription::read_from(source)?;
                let note_type = NoteType::read_from(source)?;
                Ok(OwnNoteTemplate::P2ID(payment_data, note_type))
            },
            1 => {
                let swap_data = SwapNoteDescription::read_from(source)?;
                let note_type = NoteType::read_from(source)?;
                Ok(OwnNoteTemplate::Swap(swap_data, note_type))
            },
            2 => {
                let note: Note = Note::read_from(source)?;
                Ok(OwnNoteTemplate::Note(note))
            },
            _ => {
                Err(DeserializationError::InvalidValue("invalid SendAssetNoteTemplate type".into()))
            },
        }
    }
}

/// Specifies a transaction request that can be executed by an account.
///
/// A request contains information about input notes to be consumed by the transaction (if any),
/// description of the transaction script to be executed (if any), and a set of notes expected
/// to be generated by the transaction or by consuming notes generated by the transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionRequest {
    /// Notes to be consumed by the transaction that aren't authenticated.
    unauthenticated_input_notes: Vec<Note>,
    /// Notes to be consumed by the transaction together with their (optional) arguments. This
    /// includes both authenticated and unauthenticated notes.
    input_notes: BTreeMap<NoteId, Option<NoteArgs>>,
    /// Template for the creation of the transaction script.
    script_template: Option<TransactionScriptTemplate>,
    /// A map of expected recipients for notes created by the transaction.
    expected_output_notes: BTreeMap<Digest, NoteRecipient>,
    /// A map of details and tags of notes we expect to be created as part of future transactions
    /// with their respective tags.
    ///
    /// For example, after a swap note is consumed, a payback note is expected to be created.
    expected_future_notes: BTreeMap<NoteId, (NoteDetails, NoteTag)>,
    /// Initial state of the `AdviceMap` that provides data during runtime.
    advice_map: AdviceMap,
    /// Initial state of the `MerkleStore` that provides data during runtime.
    merkle_store: MerkleStore,
    /// Foreign account data requirements. At execution time, account data will be retrieved from
    /// the network, and injected as advice inputs. Additionally, the account's code will be
    /// added to the executor and prover.
    foreign_accounts: BTreeSet<ForeignAccount>,
    /// The number of blocks in relation to the transaction's reference block after which the
    /// transaction will expire.
    expiration_delta: Option<u16>,
}

impl TransactionRequest {
    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the transaction request's unauthenticated note list.
    pub fn unauthenticated_input_notes(&self) -> &[Note] {
        &self.unauthenticated_input_notes
    }

    /// Returns an iterator over unauthenticated note IDs for the transaction request.
    pub fn unauthenticated_input_note_ids(&self) -> impl Iterator<Item = NoteId> + '_ {
        self.unauthenticated_input_notes.iter().map(Note::id)
    }

    /// Returns an iterator over authenticated input note IDs for the transaction request.
    pub fn authenticated_input_note_ids(&self) -> impl Iterator<Item = NoteId> + '_ {
        let unauthenticated_note_ids =
            self.unauthenticated_input_note_ids().collect::<BTreeSet<_>>();

        self.input_notes()
            .iter()
            .map(|(note_id, _)| *note_id)
            .filter(move |note_id| !unauthenticated_note_ids.contains(note_id))
    }

    /// Returns a mapping for input note IDs and their optional [`NoteArgs`].
    pub fn input_notes(&self) -> &BTreeMap<NoteId, Option<NoteArgs>> {
        &self.input_notes
    }

    /// Returns a list of all input note IDs.
    pub fn get_input_note_ids(&self) -> Vec<NoteId> {
        self.input_notes.keys().copied().collect()
    }

    /// Returns a map of note IDs to their respective [`NoteArgs`]. The result will include
    /// exclusively note IDs for notes for which [`NoteArgs`] have been defined.
    pub fn get_note_args(&self) -> BTreeMap<NoteId, NoteArgs> {
        self.input_notes
            .iter()
            .filter_map(|(note, args)| args.map(|a| (*note, a)))
            .collect()
    }

    /// Returns an iterator over the expected output notes.
    pub fn expected_output_notes(&self) -> impl Iterator<Item = &NoteRecipient> {
        self.expected_output_notes.values()
    }

    /// Returns an iterator over expected future notes.
    pub fn expected_future_notes(&self) -> impl Iterator<Item = &(NoteDetails, NoteTag)> {
        self.expected_future_notes.values()
    }

    /// Returns the [`TransactionScriptTemplate`].
    pub fn script_template(&self) -> &Option<TransactionScriptTemplate> {
        &self.script_template
    }

    /// Returns the [`AdviceMap`] for the transaction request.
    pub fn advice_map(&self) -> &AdviceMap {
        &self.advice_map
    }

    /// Returns the [`MerkleStore`] for the transaction request.
    pub fn merkle_store(&self) -> &MerkleStore {
        &self.merkle_store
    }

    /// Returns the IDs of the required foreign accounts for the transaction request.
    pub fn foreign_accounts(&self) -> &BTreeSet<ForeignAccount> {
        &self.foreign_accounts
    }

    /// Converts the [`TransactionRequest`] into [`TransactionArgs`] in order to be executed by a
    /// Miden host.
    pub(super) fn into_transaction_args(
        self,
        executor_account_id: AccountId,
        tx_script: TransactionScript,
    ) -> TransactionArgs {
        let note_args = self.get_note_args();
        let TransactionRequest {
            expected_output_notes,
            advice_map,
            merkle_store,
            ..
        } = self;

        let mut tx_args = TransactionArgs::new(Some(tx_script), note_args.into(), advice_map);

        tx_args.extend_output_note_recipients(expected_output_notes.into_values().map(Box::new));
        if let Some(SendNotes(own_notes)) = self.script_template {
            for own_note in own_notes {
                let note = own_note.get_outgoing_note(executor_account_id);
                tx_args.add_output_note_recipient(note);
            }
        }

        tx_args.extend_merkle_store(merkle_store.inner_nodes());

        tx_args
    }

    /// Builds the transaction script based on the account capabilities and the transaction request.
    /// The debug mode enables the script debug logs.
    pub(crate) fn build_transaction_script(
        &self,
        account_interface: &AccountInterface,
        in_debug_mode: bool,
    ) -> Result<TransactionScript, TransactionRequestError> {
        match &self.script_template {
            Some(TransactionScriptTemplate::CustomScript(script)) => Ok(script.clone()),
            Some(TransactionScriptTemplate::SendNotes(payment_templates)) => {
                let notes: Vec<PartialNote> = payment_templates
                    .iter()
                    .map(|template| template.get_outgoing_note(*account_interface.id()).into())
                    .collect();

                Ok(account_interface.build_send_notes_script(
                    &notes,
                    self.expiration_delta,
                    in_debug_mode,
                )?)
            },
            None => {
                if self.input_notes.is_empty() {
                    Err(TransactionRequestError::NoInputNotes)
                } else {
                    Ok(account_interface.build_auth_script(in_debug_mode)?)
                }
            },
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for TransactionRequest {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.unauthenticated_input_notes.write_into(target);
        self.input_notes.write_into(target);
        match &self.script_template {
            None => target.write_u8(0),
            Some(TransactionScriptTemplate::CustomScript(script)) => {
                target.write_u8(1);
                script.write_into(target);
            },
            Some(TransactionScriptTemplate::SendNotes(notes)) => {
                target.write_u8(2);
                notes.write_into(target);
            },
        }
        self.expected_output_notes.write_into(target);
        self.expected_future_notes.write_into(target);
        self.advice_map.clone().into_iter().collect::<Vec<_>>().write_into(target);
        self.merkle_store.write_into(target);
        self.foreign_accounts.write_into(target);
        self.expiration_delta.write_into(target);
    }
}

impl Deserializable for TransactionRequest {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let unauthenticated_input_notes = Vec::<Note>::read_from(source)?;
        let input_notes = BTreeMap::<NoteId, Option<NoteArgs>>::read_from(source)?;

        let script_template = match source.read_u8()? {
            0 => None,
            1 => {
                let transaction_script = TransactionScript::read_from(source)?;
                Some(TransactionScriptTemplate::CustomScript(transaction_script))
            },
            2 => {
                let notes = Vec::<OwnNoteTemplate>::read_from(source)?;
                Some(TransactionScriptTemplate::SendNotes(notes))
            },
            _ => {
                return Err(DeserializationError::InvalidValue(
                    "Invalid script template type".to_string(),
                ));
            },
        };

        let expected_output_notes = BTreeMap::<Digest, NoteRecipient>::read_from(source)?;
        let expected_future_notes = BTreeMap::<NoteId, (NoteDetails, NoteTag)>::read_from(source)?;

        let mut advice_map = AdviceMap::new();
        let advice_vec = Vec::<(Digest, Vec<Felt>)>::read_from(source)?;
        advice_map.extend(advice_vec);
        let merkle_store = MerkleStore::read_from(source)?;
        let foreign_accounts = BTreeSet::<ForeignAccount>::read_from(source)?;
        let expiration_delta = Option::<u16>::read_from(source)?;

        Ok(TransactionRequest {
            unauthenticated_input_notes,
            input_notes,
            script_template,
            expected_output_notes,
            expected_future_notes,
            advice_map,
            merkle_store,
            foreign_accounts,
            expiration_delta,
        })
    }
}

impl Default for TransactionRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// TRANSACTION REQUEST ERROR
// ================================================================================================

// Errors related to a [TransactionRequest]
#[derive(Debug, Error)]
pub enum TransactionRequestError {
    #[error("foreign account data missing in the account proof")]
    ForeignAccountDataMissing,
    #[error("foreign account storage slot {0} is not a map type")]
    ForeignAccountStorageSlotInvalidIndex(u8),
    #[error("requested foreign account with ID {0} does not have an expected storage mode")]
    InvalidForeignAccountId(AccountId),
    #[error(
        "every authenticated note to be consumed should be committed and contain a valid inclusion proof"
    )]
    InputNoteNotAuthenticated,
    #[error("the input notes map should include keys for all provided unauthenticated input notes")]
    InputNotesMapMissingUnauthenticatedNotes,
    #[error("own notes shouldn't be of the header variant")]
    InvalidNoteVariant,
    #[error("invalid sender account id: {0}")]
    InvalidSenderAccount(AccountId),
    #[error("invalid transaction script")]
    InvalidTransactionScript(#[from] AssemblyError),
    #[error("a transaction without output notes must have at least one input note")]
    NoInputNotes,
    #[error("note not found: {0}")]
    NoteNotFound(String),
    #[error("note creation error")]
    NoteCreationError(#[from] NoteError),
    #[error("pay to id note doesn't contain at least one asset")]
    P2IDNoteWithoutAsset,
    #[error("transaction script template error: {0}")]
    ScriptTemplateError(String),
    #[error("storage slot {0} not found in account ID {1}")]
    StorageSlotNotFound(u8, AccountId),
    #[error("account interface error")]
    AccountInterfaceError(#[from] AccountInterfaceError),
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use miden_lib::{note::create_p2id_note, transaction::TransactionKernel};
    use miden_objects::{
        Digest, Felt, ZERO,
        account::{AccountBuilder, AccountId, AccountIdAnchor, AccountType},
        asset::{Asset, FungibleAsset},
        crypto::rand::{FeltRng, RpoRandomCoin},
        note::{NoteExecutionMode, NoteTag, NoteType},
        testing::{
            account_component::AccountMockComponent,
            account_id::{
                ACCOUNT_ID_PRIVATE_FUNGIBLE_FAUCET,
                ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE, ACCOUNT_ID_SENDER,
            },
        },
    };
    use miden_tx::utils::{Deserializable, Serializable};

    use super::{
        OwnNoteTemplate, PaymentNoteDescription, TransactionRequest, TransactionRequestBuilder,
    };
    use crate::{
        rpc::domain::account::AccountStorageRequirements,
        transaction::{ForeignAccount, ForeignAccountInputs, SwapNoteDescription},
    };

    #[test]
    fn transaction_request_serialization() {
        let sender_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
        let target_id =
            AccountId::try_from(ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE).unwrap();
        let faucet_id = AccountId::try_from(ACCOUNT_ID_PRIVATE_FUNGIBLE_FAUCET).unwrap();
        let mut rng = RpoRandomCoin::new(Default::default());

        let mut notes = vec![];
        for i in 0..6 {
            let note = create_p2id_note(
                sender_id,
                target_id,
                vec![FungibleAsset::new(faucet_id, 100 + i).unwrap().into()],
                NoteType::Private,
                ZERO,
                &mut rng,
            )
            .unwrap();
            notes.push(note);
        }

        let mut advice_vec: Vec<(Digest, Vec<Felt>)> = vec![];
        for i in 0..10 {
            advice_vec.push((Digest::new(rng.draw_word()), vec![Felt::new(i)]));
        }

        let account = AccountBuilder::new(Default::default())
            .anchor(AccountIdAnchor::new_unchecked(0, Digest::default()))
            .with_component(
                AccountMockComponent::new_with_empty_slots(TransactionKernel::assembler()).unwrap(),
            )
            .account_type(AccountType::RegularAccountImmutableCode)
            .storage_mode(miden_objects::account::AccountStorageMode::Private)
            .build_existing()
            .unwrap();

        // This transaction request wouldn't be valid in a real scenario, it's intended for testing
        let tx_request = TransactionRequestBuilder::new()
            .with_authenticated_input_notes(vec![(notes.pop().unwrap().id(), None)])
            .with_unauthenticated_input_notes(vec![(notes.pop().unwrap(), None)])
            .extend_expected_output_notes([notes.pop().unwrap().recipient().clone()])
            .extend_expected_future_notes(vec![(
                notes.pop().unwrap().into(),
                NoteTag::from_account_id(sender_id, NoteExecutionMode::Local).unwrap(),
            )])
            .extend_advice_map(advice_vec)
            .with_foreign_accounts([
                ForeignAccount::public(
                    target_id,
                    AccountStorageRequirements::new([(5u8, &[Digest::default()])]),
                )
                .unwrap(),
                ForeignAccount::private(
                    ForeignAccountInputs::from_account(
                        account,
                        &AccountStorageRequirements::default(),
                    )
                    .unwrap(),
                )
                .unwrap(),
            ])
            .extend_own_output_notes(vec![
                OwnNoteTemplate::P2ID(
                    PaymentNoteDescription::new(
                        vec![Asset::Fungible(FungibleAsset::new(faucet_id, 100).unwrap())],
                        target_id,
                        Some(123.into()),
                        &mut rng,
                    )
                    .unwrap(),
                    NoteType::Public,
                ),
                OwnNoteTemplate::Swap(
                    SwapNoteDescription::new(
                        Asset::Fungible(FungibleAsset::new(faucet_id, 100).unwrap()),
                        Asset::Fungible(FungibleAsset::new(faucet_id, 100).unwrap()),
                        &mut rng,
                    ),
                    NoteType::Public,
                ),
            ])
            .build()
            .unwrap();

        let mut buffer = Vec::new();
        tx_request.write_into(&mut buffer);

        let deserialized_tx_request = TransactionRequest::read_from_bytes(&buffer).unwrap();
        assert_eq!(tx_request, deserialized_tx_request);
    }
}
