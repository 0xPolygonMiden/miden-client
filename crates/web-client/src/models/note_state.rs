use miden_client::store::NoteState as NativeNoteState;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
#[wasm_bindgen]
pub enum NoteState {
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

impl From<NativeNoteState> for NoteState {
    fn from(native_note: NativeNoteState) -> Self {
        match native_note {
            NativeNoteState::Expected(_) => NoteState::Expected,
            NativeNoteState::Unverified(_) => NoteState::Unverified,
            NativeNoteState::Committed(_) => NoteState::Committed,
            NativeNoteState::Invalid(_) => NoteState::Invalid,
            NativeNoteState::ProcessingAuthenticated(_) => NoteState::ProcessingAuthenticated,
            NativeNoteState::ProcessingUnauthenticated(_) => NoteState::ProcessingUnauthenticated,
            NativeNoteState::ConsumedAuthenticatedLocal(_) => NoteState::ConsumedAuthenticatedLocal,
            NativeNoteState::ConsumedUnauthenticatedLocal(_) => {
                NoteState::ConsumedUnauthenticatedLocal
            },
            NativeNoteState::ConsumedExternal(_) => NoteState::ConsumedExternal,
        }
    }
}

impl From<&NativeNoteState> for NoteState {
    fn from(native_note: &NativeNoteState) -> Self {
        match native_note {
            NativeNoteState::Expected(_) => NoteState::Expected,
            NativeNoteState::Unverified(_) => NoteState::Unverified,
            NativeNoteState::Committed(_) => NoteState::Committed,
            NativeNoteState::Invalid(_) => NoteState::Invalid,
            NativeNoteState::ProcessingAuthenticated(_) => NoteState::ProcessingAuthenticated,
            NativeNoteState::ProcessingUnauthenticated(_) => NoteState::ProcessingUnauthenticated,
            NativeNoteState::ConsumedAuthenticatedLocal(_) => NoteState::ConsumedAuthenticatedLocal,
            NativeNoteState::ConsumedUnauthenticatedLocal(_) => {
                NoteState::ConsumedUnauthenticatedLocal
            },
            NativeNoteState::ConsumedExternal(_) => NoteState::ConsumedExternal,
        }
    }
}
