//! Contains structures and functions related to transaction creation.
use alloc::{collections::BTreeMap, string::ToString, vec::Vec};

use miden_lib::note::{create_p2id_note, create_p2idr_note, create_swap_note};
use miden_objects::{
    Digest, Felt, FieldElement,
    account::AccountId,
    asset::{Asset, FungibleAsset},
    block::BlockNumber,
    crypto::merkle::{InnerNodeInfo, MerkleStore},
    note::{Note, NoteDetails, NoteExecutionMode, NoteId, NoteTag, NoteType, PartialNote},
    transaction::{OutputNote, TransactionScript},
    vm::AdviceMap,
};

use super::{
    ForeignAccount, NoteArgs, TransactionRequest, TransactionRequestError,
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
    own_output_notes: Vec<OutputNote>,
    /// A map of expected full output notes to be generated by the transaction.
    expected_output_notes: BTreeMap<NoteId, Note>,
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
    pub fn with_own_output_notes(mut self, notes: impl IntoIterator<Item = OutputNote>) -> Self {
        for note in notes {
            if let OutputNote::Full(note) = &note {
                self.expected_output_notes.insert(note.id(), note.clone());
            }

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

    /// Specifies a transaction's expected output notes.
    ///
    /// The set of specified notes is treated as a subset of the notes that may be created by a
    /// transaction. That is, the transaction must create all the specified expected notes, but it
    /// may also create other notes which aren't included in the set of expected notes.
    #[must_use]
    pub fn with_expected_output_notes(mut self, notes: Vec<Note>) -> Self {
        self.expected_output_notes =
            notes.into_iter().map(|note| (note.id(), note)).collect::<BTreeMap<_, _>>();
        self
    }

    /// Specifies a set of notes which may be created when a transaction's output notes are
    /// consumed.
    ///
    /// For example, after a SWAP note is consumed, a payback note is expected to be created. This
    /// allows the client to track this note accordingly.
    #[must_use]
    pub fn with_expected_future_notes(mut self, notes: Vec<(NoteDetails, NoteTag)>) -> Self {
        self.expected_future_notes =
            notes.into_iter().map(|note| (note.0.id(), note)).collect::<BTreeMap<_, _>>();
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

    /// Returns a new built [`TransactionRequest`] for a transaction to consume the specified
    /// notes.
    ///
    /// - `note_ids` is a list of note IDs to be consumed.
    pub fn build_consume_notes(
        note_ids: Vec<NoteId>,
    ) -> Result<TransactionRequest, TransactionRequestError> {
        let input_notes = note_ids.into_iter().map(|id| (id, None));
        Self::new().with_authenticated_input_notes(input_notes).build()
    }

    /// Returns a new built [`TransactionRequest`] for a transaction to mint fungible assets. This
    /// request must be executed against a fungible faucet account.
    ///
    /// - `asset` is the fungible asset to be minted.
    /// - `target_id` is the account ID of the account to receive the minted asset.
    /// - `note_type` determines the visibility of the note to be created.
    /// - `rng` is the random number generator used to generate the serial number for the created
    ///   note.
    pub fn build_mint_fungible_asset(
        asset: FungibleAsset,
        target_id: AccountId,
        note_type: NoteType,
        rng: &mut ClientRng,
    ) -> Result<Self, TransactionRequestError> {
        let created_note = create_p2id_note(
            asset.faucet_id(),
            target_id,
            vec![asset.into()],
            note_type,
            Felt::ZERO,
            rng,
        )?;

        Self::new().with_own_output_notes(vec![OutputNote::Full(created_note)]).build()
    }

    /// Returns a new built [`TransactionRequest`] for a transaction to send a P2ID or P2IDR note.
    /// This request must be executed against the wallet sender account.
    ///
    /// - `payment_data` is the data for the payment transaction that contains the asset to be
    ///   transferred, the sender account ID, and the target account ID.
    /// - `recall_height` is the block height after which the sender can recall the assets. If None,
    ///   a P2ID note is created. If `Some()`, a P2IDR note is created.
    /// - `note_type` determines the visibility of the note to be created.
    /// - `rng` is the random number generator used to generate the serial number for the created
    ///   note.
    pub fn build_pay_to_id(
        payment_data: PaymentTransactionData,
        recall_height: Option<BlockNumber>,
        note_type: NoteType,
        rng: &mut ClientRng,
    ) -> Result<Self, TransactionRequestError> {
        let PaymentTransactionData {
            assets,
            sender_account_id,
            target_account_id,
        } = payment_data;

        if assets
            .iter()
            .all(|asset| asset.is_fungible() && asset.unwrap_fungible().amount() == 0)
        {
            return Err(TransactionRequestError::P2IDNoteWithoutAsset);
        }

        let created_note = if let Some(recall_height) = recall_height {
            create_p2idr_note(
                sender_account_id,
                target_account_id,
                assets,
                note_type,
                Felt::ZERO,
                recall_height,
                rng,
            )?
        } else {
            create_p2id_note(
                sender_account_id,
                target_account_id,
                assets,
                note_type,
                Felt::ZERO,
                rng,
            )?
        };

        Self::new().with_own_output_notes(vec![OutputNote::Full(created_note)]).build()
    }

    /// Returns a new built [`TransactionRequest`] for a transaction to send a SWAP note. This
    /// request must be executed against the wallet sender account.
    ///
    /// - `swap_data` is the data for the swap transaction that contains the sender account ID, the
    ///   offered asset, and the requested asset.
    /// - `note_type` determines the visibility of the note to be created.
    /// - `rng` is the random number generator used to generate the serial number for the created
    ///   note.
    pub fn build_swap(
        swap_data: &SwapTransactionData,
        note_type: NoteType,
        rng: &mut ClientRng,
    ) -> Result<Self, TransactionRequestError> {
        // The created note is the one that we need as the output of the tx, the other one is the
        // one that we expect to receive and consume eventually.
        let (created_note, payback_note_details) = create_swap_note(
            swap_data.account_id(),
            swap_data.offered_asset(),
            swap_data.requested_asset(),
            note_type,
            Felt::ZERO,
            rng,
        )?;

        let payback_tag =
            NoteTag::from_account_id(swap_data.account_id(), NoteExecutionMode::Local)?;

        Self::new()
            .with_expected_future_notes(vec![(payback_note_details, payback_tag)])
            .with_own_output_notes(vec![OutputNote::Full(created_note)])
            .build()
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
            (None, false) => {
                let partial_notes = self
                    .own_output_notes
                    .into_iter()
                    .map(|note| match note {
                        OutputNote::Header(_) => Err(TransactionRequestError::InvalidNoteVariant),
                        OutputNote::Partial(note) => Ok(note),
                        OutputNote::Full(note) => Ok(note.into()),
                    })
                    .collect::<Result<Vec<PartialNote>, _>>()?;

                Some(TransactionScriptTemplate::SendNotes(partial_notes))
            },
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
#[derive(Clone, Debug)]
pub struct PaymentTransactionData {
    /// Assets that are meant to be sent to the target account.
    assets: Vec<Asset>,
    /// Account ID of the sender account.
    sender_account_id: AccountId,
    /// Account ID of the receiver account.
    target_account_id: AccountId,
}

impl PaymentTransactionData {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`PaymentTransactionData`].
    pub fn new(
        assets: Vec<Asset>,
        sender_account_id: AccountId,
        target_account_id: AccountId,
    ) -> PaymentTransactionData {
        PaymentTransactionData {
            assets,
            sender_account_id,
            target_account_id,
        }
    }

    /// Returns the executor [`AccountId`].
    pub fn account_id(&self) -> AccountId {
        self.sender_account_id
    }

    /// Returns the target [`AccountId`].
    pub fn target_account_id(&self) -> AccountId {
        self.target_account_id
    }

    /// Returns the transaction's list of [`Asset`].
    pub fn assets(&self) -> &Vec<Asset> {
        &self.assets
    }
}

// SWAP TRANSACTION DATA
// ================================================================================================

/// Contains information related to a swap transaction.
///
/// A swap transaction involves creating a SWAP note, which will carry the offered asset and which,
/// when consumed, will create a payback note that carries the requested asset taken from the
/// consumer account's vault.
#[derive(Clone, Debug)]
pub struct SwapTransactionData {
    /// Account ID of the sender account.
    sender_account_id: AccountId,
    /// Asset that is offered in the swap.
    offered_asset: Asset,
    /// Asset that is expected in the payback note generated as a result of the swap.
    requested_asset: Asset,
}

impl SwapTransactionData {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`SwapTransactionData`].
    pub fn new(
        sender_account_id: AccountId,
        offered_asset: Asset,
        requested_asset: Asset,
    ) -> SwapTransactionData {
        SwapTransactionData {
            sender_account_id,
            offered_asset,
            requested_asset,
        }
    }

    /// Returns the executor [`AccountId`].
    pub fn account_id(&self) -> AccountId {
        self.sender_account_id
    }

    /// Returns the transaction offered [`Asset`].
    pub fn offered_asset(&self) -> Asset {
        self.offered_asset
    }

    /// Returns the transaction requested [`Asset`].
    pub fn requested_asset(&self) -> Asset {
        self.requested_asset
    }
}
