use miden_client::store::InputNoteState as NativeNoteState;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub enum InputNoteState {
    Expected,
    Unverified,
    Committed,
    Invalid,
    ProcessingAuthenticated,
    ProcessingUnauthenticated,
    ConsumedAuthenticatedLocal,
    ConsumedUnauthenticatedLocal,
    ConsumedExternal,
}

// CONVERSIONS
// ================================================================================================

impl From<NativeNoteState> for InputNoteState {
    fn from(native_note: NativeNoteState) -> Self {
        match native_note {
            NativeNoteState::Expected(_) => InputNoteState::Expected,
            NativeNoteState::Unverified(_) => InputNoteState::Unverified,
            NativeNoteState::Committed(_) => InputNoteState::Committed,
            NativeNoteState::Invalid(_) => InputNoteState::Invalid,
            NativeNoteState::ProcessingAuthenticated(_) => InputNoteState::ProcessingAuthenticated,
            NativeNoteState::ProcessingUnauthenticated(_) => {
                InputNoteState::ProcessingUnauthenticated
            },
            NativeNoteState::ConsumedAuthenticatedLocal(_) => {
                InputNoteState::ConsumedAuthenticatedLocal
            },
            NativeNoteState::ConsumedUnauthenticatedLocal(_) => {
                InputNoteState::ConsumedUnauthenticatedLocal
            },
            NativeNoteState::ConsumedExternal(_) => InputNoteState::ConsumedExternal,
        }
    }
}

impl From<&NativeNoteState> for InputNoteState {
    fn from(native_note: &NativeNoteState) -> Self {
        match native_note {
            NativeNoteState::Expected(_) => InputNoteState::Expected,
            NativeNoteState::Unverified(_) => InputNoteState::Unverified,
            NativeNoteState::Committed(_) => InputNoteState::Committed,
            NativeNoteState::Invalid(_) => InputNoteState::Invalid,
            NativeNoteState::ProcessingAuthenticated(_) => InputNoteState::ProcessingAuthenticated,
            NativeNoteState::ProcessingUnauthenticated(_) => {
                InputNoteState::ProcessingUnauthenticated
            },
            NativeNoteState::ConsumedAuthenticatedLocal(_) => {
                InputNoteState::ConsumedAuthenticatedLocal
            },
            NativeNoteState::ConsumedUnauthenticatedLocal(_) => {
                InputNoteState::ConsumedUnauthenticatedLocal
            },
            NativeNoteState::ConsumedExternal(_) => InputNoteState::ConsumedExternal,
        }
    }
}
