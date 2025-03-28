//! Contains structures and functions related to transaction creation.
use alloc::{collections::BTreeMap, string::ToString, vec::Vec};

use miden_lib::note::{
    utils::{self, build_swap_tag},
    well_known_note::WellKnownNote,
};
use miden_objects::{
    Digest, Felt, FieldElement, NoteError, Word,
    account::AccountId,
    asset::{Asset, FungibleAsset},
    block::BlockNumber,
    crypto::{
        merkle::{InnerNodeInfo, MerkleStore},
        rand::FeltRng,
    },
    note::{
        Note, NoteAssets, NoteDetails, NoteExecutionHint, NoteExecutionMode, NoteId, NoteInputs,
        NoteMetadata, NoteRecipient, NoteTag, NoteType,
    },
    transaction::TransactionScript,
    vm::AdviceMap,
};
use miden_tx::utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};

use super::{
    ForeignAccount, NoteArgs, OwnNoteTemplate, TransactionRequest, TransactionRequestError,
    TransactionScriptTemplate,
};
use crate::ClientRng;

// TRANSACTION REQUEST BUILDER
// ================================================================================================

/// A builder for a [`TransactionRequest`].
///
/// Use this builder to construct a [`TransactionRequest`] by adding input notes, specifying
/// scripts, and setting other transaction parameters.
#[derive(Clone, Debug)]
pub struct TransactionRequestBuilder {
    /// Notes to be consumed by the transaction that aren't authenticated.
    unauthenticated_input_notes: Vec<Note>,
    /// Notes to be consumed by the transaction together with their (optional) arguments. This
    /// includes both authenticated and unauthenticated notes.
    input_notes: BTreeMap<NoteId, Option<NoteArgs>>,
    /// Notes to be created by the transaction. This includes both full and partial output notes.
    /// The transaction script will be generated based on these notes.
    own_output_notes: Vec<OwnNoteTemplate>,
    /// A map of expected recipients for notes created by the transaction.
    expected_output_notes: BTreeMap<Digest, NoteRecipient>,
    /// A map of details and tags of notes we expect to be created as part of future transactions
    /// with their respective tags.
    ///
    /// For example, after a swap note is consumed, a payback note is expected to be created.
    expected_future_notes: BTreeMap<NoteId, (NoteDetails, NoteTag)>,
    /// Custom transaction script to be used.
    custom_script: Option<TransactionScript>,
    /// Initial state of the `AdviceMap` that provides data during runtime.
    advice_map: AdviceMap,
    /// Initial state of the `MerkleStore` that provides data during runtime.
    merkle_store: MerkleStore,
    /// Foreign account data requirements. At execution time, account data will be retrieved from
    /// the network, and injected as advice inputs. Additionally, the account's code will be
    /// added to the executor and prover.
    foreign_accounts: BTreeMap<AccountId, ForeignAccount>,
    /// The number of blocks in relation to the transaction's reference block after which the
    /// transaction will expire.
    expiration_delta: Option<u16>,
}

impl TransactionRequestBuilder {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new, empty [`TransactionRequestBuilder`].
    pub fn new() -> Self {
        Self {
            unauthenticated_input_notes: vec![],
            input_notes: BTreeMap::new(),
            own_output_notes: Vec::new(),
            expected_output_notes: BTreeMap::new(),
            expected_future_notes: BTreeMap::new(),
            custom_script: None,
            advice_map: AdviceMap::default(),
            merkle_store: MerkleStore::default(),
            expiration_delta: None,
            foreign_accounts: BTreeMap::default(),
        }
    }

    /// Adds the specified notes as unauthenticated input notes to the transaction request.
    #[must_use]
    pub fn with_unauthenticated_input_notes(
        mut self,
        notes: impl IntoIterator<Item = (Note, Option<NoteArgs>)>,
    ) -> Self {
        for (note, argument) in notes {
            self.input_notes.insert(note.id(), argument);
            self.unauthenticated_input_notes.push(note);
        }
        self
    }

    /// Adds the specified notes as authenticated input notes to the transaction request.
    #[must_use]
    pub fn with_authenticated_input_notes(
        mut self,
        notes: impl IntoIterator<Item = (NoteId, Option<NoteArgs>)>,
    ) -> Self {
        for (note_id, argument) in notes {
            self.input_notes.insert(note_id, argument);
        }
        self
    }

    /// Specifies the output notes that should be created in the transaction script and will
    /// be used as a transaction script template. These notes will also be added to the expected
    /// output notes of the transaction.
    ///
    /// If a transaction script template is already set (e.g. by calling `with_custom_script`), the
    /// [`TransactionRequestBuilder::build`] method will return an error.
    #[must_use]
    pub fn extend_own_output_notes(
        mut self,
        notes: impl IntoIterator<Item = OwnNoteTemplate>,
    ) -> Self {
        for note in notes {
            self.own_output_notes.push(note);
        }

        self
    }

    /// Specifies a custom transaction script to be used.
    ///
    /// If a script template is already set (e.g. by calling `with_own_output_notes`), the
    /// [`TransactionRequestBuilder::build`] method will return an error.
    #[must_use]
    pub fn with_custom_script(mut self, script: TransactionScript) -> Self {
        self.custom_script = Some(script);
        self
    }

    /// Specifies one or more foreign accounts (public or private) that contain data
    /// utilized by the transaction.
    ///
    /// At execution, the client queries the node and retrieves the appropriate data,
    /// depending on whether each foreign account is public or private:
    ///
    /// - **Public accounts**: the node retrieves the state and code for the account and injects
    ///   them as advice inputs.
    /// - **Private accounts**: the node retrieves a proof of the account's existence and injects
    ///   that as advice inputs.
    #[must_use]
    pub fn with_foreign_accounts(
        mut self,
        foreign_accounts: impl IntoIterator<Item = impl Into<ForeignAccount>>,
    ) -> Self {
        for account in foreign_accounts {
            let foreign_account: ForeignAccount = account.into();
            self.foreign_accounts.insert(foreign_account.account_id(), foreign_account);
        }

        self
    }

    /// Inserts notes into the set of expected output notes.
    ///
    /// The set of specified notes is treated as a subset of the notes that may be created by a
    /// transaction. That is, the transaction must create all the specified expected notes, but it
    /// may also create other notes which aren't included in the set of expected notes.
    #[must_use]
    pub fn extend_expected_output_notes(
        mut self,
        note_recipients: impl IntoIterator<Item = NoteRecipient>,
    ) -> Self {
        for recipient in note_recipients {
            self.expected_output_notes.insert(recipient.digest(), recipient);
        }
        self
    }

    /// Inserts notes into the set of notes which may be created when this transaction's output
    /// notes are consumed.
    ///
    /// For example, after a SWAP note is consumed, a payback note is expected to be created. This
    /// allows the client to track this note accordingly.
    #[must_use]
    pub fn extend_expected_future_notes(
        mut self,
        notes: impl IntoIterator<Item = (NoteDetails, NoteTag)>,
    ) -> Self {
        for (note_details, note_tag) in notes {
            self.expected_future_notes.insert(note_details.id(), (note_details, note_tag));
        }
        self
    }

    /// Extends the advice map with the specified `([Digest], Vec<[Felt]>)` pairs.
    #[must_use]
    pub fn extend_advice_map<T: IntoIterator<Item = (Digest, Vec<Felt>)>>(
        mut self,
        iter: T,
    ) -> Self {
        self.advice_map.extend(iter);
        self
    }

    /// Extends the merkle store with the specified [`InnerNodeInfo`] elements.
    #[must_use]
    pub fn extend_merkle_store<T: IntoIterator<Item = InnerNodeInfo>>(mut self, iter: T) -> Self {
        self.merkle_store.extend(iter);
        self
    }

    /// The number of blocks in relation to the transaction's reference block after which the
    /// transaction will expire.
    ///
    /// Setting transaction expiration delta defines an upper bound for transaction expiration,
    /// but other code executed during the transaction may impose an even smaller transaction
    /// expiration delta.
    #[must_use]
    pub fn with_expiration_delta(mut self, expiration_delta: u16) -> Self {
        self.expiration_delta = Some(expiration_delta);
        self
    }

    // STANDARDIZED REQUESTS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [`TransactionRequestBuilder`] for a transaction to consume the specified
    /// notes.
    ///
    /// - `note_ids` is a list of note IDs to be consumed.
    pub fn consume_notes(note_ids: Vec<NoteId>) -> Self {
        let input_notes = note_ids.into_iter().map(|id| (id, None));
        Self::new().with_authenticated_input_notes(input_notes)
    }

    /// Returns a new [`TransactionRequestBuilder`] for a transaction to mint fungible assets. This
    /// request must be executed against a fungible faucet account.
    ///
    /// - `asset` is the fungible asset to be minted.
    /// - `target_id` is the account ID of the account to receive the minted asset.
    /// - `note_type` determines the visibility of the note to be created.
    /// - `rng` is the random number generator used to generate the serial number for the created
    ///   note.
    pub fn mint_fungible_asset(
        asset: FungibleAsset,
        target_id: AccountId,
        note_type: NoteType,
        rng: &mut ClientRng,
    ) -> Result<Self, TransactionRequestError> {
        let payment_transaction_data =
            PaymentNoteDescription::new(vec![Asset::Fungible(asset)], target_id, None, rng)?;

        Ok(Self::new().extend_own_output_notes(vec![OwnNoteTemplate::P2ID(
            payment_transaction_data,
            note_type,
        )]))
    }

    /// Returns a new [`TransactionRequestBuilder`] for a transaction to send a P2ID or P2IDR note.
    /// This request must be executed against the wallet sender account.
    ///
    /// - `payment_data` is the data for the payment transaction that contains the asset to be
    ///   transferred, the sender account ID, and the target account ID.
    /// - `recall_height` is the block height after which the sender can recall the assets. If None,
    ///   a P2ID note is created. If `Some()`, a P2IDR note is created.
    /// - `note_type` determines the visibility of the note to be created.
    /// - `rng` is the random number generator used to generate the serial number for the created
    ///   note.
    pub fn pay_to_id(
        assets: Vec<Asset>,
        target_account_id: AccountId,
        recall_height: Option<BlockNumber>,
        note_type: NoteType,
        rng: &mut ClientRng,
    ) -> Result<Self, TransactionRequestError> {
        if assets
            .iter()
            .all(|asset| asset.is_fungible() && asset.unwrap_fungible().amount() == 0)
        {
            return Err(TransactionRequestError::P2IDNoteWithoutAsset);
        }

        let payment_data =
            PaymentNoteDescription::new(assets, target_account_id, recall_height, rng)?;

        Ok(Self::new().extend_own_output_notes([OwnNoteTemplate::P2ID(payment_data, note_type)]))
    }

    /// Returns a new [`TransactionRequestBuilder`] for a transaction to send a SWAP note. This
    /// request must be executed against the wallet sender account.
    ///
    /// - `offered_asset`: Asset that is offered in the swap.
    /// - `requested_asset`: Asset that is requested as part of the swap. Once the outgoing note is
    ///   consumed, a payback note with this asset will be created.
    /// - `note_type` determines the visibility of the note to be created.
    pub fn swap(
        offered_asset: Asset,
        requested_asset: Asset,
        note_type: NoteType,
        rng: &mut ClientRng,
    ) -> Result<Self, TransactionRequestError> {
        let swap_data = SwapNoteDescription::new(offered_asset, requested_asset, rng);

        Ok(Self::new().extend_own_output_notes(vec![OwnNoteTemplate::Swap(swap_data, note_type)]))
    }

    // FINALIZE BUILDER
    // --------------------------------------------------------------------------------------------

    /// Consumes the builder and returns a [`TransactionRequest`].
    ///
    /// # Errors
    /// - If both a custom script and own output notes are set.
    /// - If an expiration delta is set when a custom script is set.
    /// - If an invalid note variant is encountered in the own output notes.
    pub fn build(self) -> Result<TransactionRequest, TransactionRequestError> {
        let script_template = match (self.custom_script, self.own_output_notes.is_empty()) {
            (Some(_), false) => {
                return Err(TransactionRequestError::ScriptTemplateError(
                    "Cannot set both a custom script and own output notes".to_string(),
                ));
            },
            (Some(script), true) => {
                if self.expiration_delta.is_some() {
                    return Err(TransactionRequestError::ScriptTemplateError(
                        "Cannot set expiration delta when a custom script is set".to_string(),
                    ));
                }

                Some(TransactionScriptTemplate::CustomScript(script))
            },
            (None, false) => Some(TransactionScriptTemplate::SendNotes(self.own_output_notes)),
            (None, true) => None,
        };

        Ok(TransactionRequest {
            unauthenticated_input_notes: self.unauthenticated_input_notes,
            input_notes: self.input_notes,
            script_template,
            expected_output_notes: self.expected_output_notes,
            expected_future_notes: self.expected_future_notes,
            advice_map: self.advice_map,
            merkle_store: self.merkle_store,
            foreign_accounts: self.foreign_accounts.into_values().collect(),
            expiration_delta: self.expiration_delta,
        })
    }
}

// PAYMENT TRANSACTION DATA
// ================================================================================================

/// Contains information about a payment transaction.
///
/// This struct is a sender-agnostic description of an own note that can be output as part of a
/// [`TransactionRequest`].
///
/// If a `recall_height` is set, a P2IDR (Pay-to-ID with recall) note script will be generated.
/// Otherwise, a P2ID (Pay-to-ID) note script is output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentNoteDescription {
    /// Assets that are meant to be sent to the target account.
    assets: Vec<Asset>,
    /// Account ID of the receiver account.
    target_account_id: AccountId,
    /// Determines at which block height the note could be recalled by the sender.
    recall_height: Option<BlockNumber>,
    /// Serial number of the generated note
    serial_num: Word,
}

impl PaymentNoteDescription {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`PaymentNoteDescription`].
    pub fn new(
        assets: Vec<Asset>,
        target_account_id: AccountId,
        recall_height: Option<BlockNumber>,
        rng: &mut impl FeltRng,
    ) -> Result<PaymentNoteDescription, NoteError> {
        // TODO: Replace with the actual const
        if assets.len() > 255 {
            return Err(NoteError::TooManyAssets(assets.len()));
        }

        Ok(PaymentNoteDescription {
            assets,
            target_account_id,
            recall_height,
            serial_num: rng.draw_word(),
        })
    }

    /// Returns the target [`AccountId`].
    pub fn target_account_id(&self) -> AccountId {
        self.target_account_id
    }

    /// Returns the transaction's list of [`Asset`].
    pub fn assets(&self) -> &Vec<Asset> {
        &self.assets
    }

    /// Returns the block height in which the note can be recalled by the sender.
    pub fn recall_height(&self) -> Option<BlockNumber> {
        self.recall_height
    }

    /// Creates the output [`Note`] object based on the sender ID.
    pub(crate) fn get_note(&self, sender_account_id: AccountId, note_type: NoteType) -> Note {
        if let Some(recall_block_number) = self.recall_height {
            let note_script = WellKnownNote::P2IDR.script();

            let inputs = NoteInputs::new(vec![
                self.target_account_id.suffix(),
                self.target_account_id.prefix().as_felt(),
                recall_block_number.into(),
            ])
            .expect("good by construction (small input number)");
            let tag = NoteTag::from_account_id(self.target_account_id, NoteExecutionMode::Local)
                .expect("good by construction");

            let vault = NoteAssets::new(self.assets.clone()).expect("validated on Self::new()");
            let metadata = NoteMetadata::new(
                sender_account_id,
                note_type,
                tag,
                NoteExecutionHint::always(),
                Felt::ZERO,
            )
            .expect("valid tag");
            let recipient = NoteRecipient::new(self.serial_num, note_script, inputs);
            Note::new(vault, metadata, recipient)
        } else {
            let recipient = utils::build_p2id_recipient(self.target_account_id, self.serial_num)
                .expect("inputs validated on construction");

            let tag = NoteTag::from_account_id(self.target_account_id, NoteExecutionMode::Local)
                .expect("valid tag (local execution mode)");

            let metadata = NoteMetadata::new(
                sender_account_id,
                note_type,
                tag,
                NoteExecutionHint::always(),
                Felt::ZERO,
            )
            .expect("valid tag");
            let vault = NoteAssets::new(self.assets.clone()).expect("validated on Self::new()");

            Note::new(vault, metadata, recipient)
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for PaymentNoteDescription {
    fn write_into<W: miden_tx::utils::ByteWriter>(&self, target: &mut W) {
        target.write(self.assets());
        target.write(self.target_account_id());
        target.write(self.recall_height);
        target.write(self.serial_num);
    }
}

impl Deserializable for PaymentNoteDescription {
    fn read_from<R: miden_tx::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_tx::utils::DeserializationError> {
        let assets: Vec<Asset> = source.read()?;
        let target_account_id: AccountId = source.read()?;
        let recall_height: Option<BlockNumber> = source.read()?;
        let serial_num: Word = source.read()?;

        Ok(PaymentNoteDescription {
            assets,
            target_account_id,
            recall_height,
            serial_num,
        })
    }
}

// SWAP TRANSACTION DATA
// ================================================================================================

/// Contains information related to a swap transaction.
///
/// A swap transaction involves creating a SWAP note, which will carry the offered asset and which,
/// when consumed, will create a payback note that carries the requested asset taken from the
/// consumer account's vault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapNoteDescription {
    /// Asset that is offered in the swap.
    offered_asset: Asset,
    /// Asset that is expected in the payback note generated as a result of the swap.
    requested_asset: Asset,
    /// Serial number of the outgoing note.
    serial_num: Word,
    /// Serial number of the payback note.
    payback_serial_num: Word,
}

impl SwapNoteDescription {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`SwapNoteDescription`].
    pub fn new(
        offered_asset: Asset,
        requested_asset: Asset,
        rng: &mut impl FeltRng,
    ) -> SwapNoteDescription {
        SwapNoteDescription {
            offered_asset,
            requested_asset,
            serial_num: rng.draw_word(),
            payback_serial_num: rng.draw_word(),
        }
    }

    /// Returns the transaction offered [`Asset`].
    pub fn offered_asset(&self) -> Asset {
        self.offered_asset
    }

    /// Returns the transaction requested [`Asset`].
    pub fn requested_asset(&self) -> Asset {
        self.requested_asset
    }

    /// Returns the swap note and the future payback note with its tag.
    pub fn get_note(
        &self,
        sender_account_id: AccountId,
        note_type: NoteType,
    ) -> (Note, NoteDetails, NoteTag) {
        let note_script = WellKnownNote::SWAP.script();

        let payback_recipient =
            utils::build_p2id_recipient(sender_account_id, self.payback_serial_num)
                .expect("input number is valid");

        let payback_recipient_word: Word = payback_recipient.digest().into();
        let requested_asset_word: Word = self.requested_asset.into();
        let payback_tag = NoteTag::from_account_id(sender_account_id, NoteExecutionMode::Local)
            .expect("valid tag");

        let inputs = NoteInputs::new(vec![
            payback_recipient_word[0],
            payback_recipient_word[1],
            payback_recipient_word[2],
            payback_recipient_word[3],
            requested_asset_word[0],
            requested_asset_word[1],
            requested_asset_word[2],
            requested_asset_word[3],
            payback_tag.inner().into(),
            NoteExecutionHint::always().into(),
        ])
        .expect("valid note inputs length");

        // build the tag for the SWAP use case
        let tag = build_swap_tag(note_type, &self.offered_asset, &self.requested_asset)
            .expect("valid tag");

        // build the outgoing note
        let metadata = NoteMetadata::new(
            sender_account_id,
            note_type,
            tag,
            NoteExecutionHint::always(),
            Felt::ZERO,
        )
        .expect("valid tag");
        let assets = NoteAssets::new(vec![self.offered_asset]).expect("one asset is valid");
        let recipient = NoteRecipient::new(self.serial_num, note_script, inputs);
        let note = Note::new(assets, metadata, recipient);

        // build the payback note details
        let payback_assets =
            NoteAssets::new(vec![self.requested_asset]).expect("one asset is valid");
        let payback_note = NoteDetails::new(payback_assets, payback_recipient);

        (note, payback_note, tag)
    }
}

impl Serializable for SwapNoteDescription {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.offered_asset.write_into(target);
        self.requested_asset.write_into(target);
        self.serial_num.write_into(target);
        self.payback_serial_num.write_into(target);
    }
}

impl Deserializable for SwapNoteDescription {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let offered_asset = Asset::read_from(source)?;
        let requested_asset = Asset::read_from(source)?;
        let serial_num = Word::read_from(source)?;
        let payback_serial_num = Word::read_from(source)?;
        Ok(SwapNoteDescription {
            offered_asset,
            requested_asset,
            serial_num,
            payback_serial_num,
        })
    }
}
