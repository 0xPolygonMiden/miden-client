import wasm from "../dist/wasm.js";

const {
    AccountId,
    AdviceMap,
    AuthSecretKey,
    Felt,
    FeltArray,
    FungibleAsset,
    Note,
    NoteAssets,
    NoteExecutionHint,
    NoteExecutionMode,
    NoteId,
    NoteIdAndArgs,
    NoteIdAndArgsArray,
    NoteInputs,
    NoteMetadata,
    NoteRecipient,
    NoteTag,
    NoteType,
    OutputNote,
    OutputNotesArray,
    Rpo256,
    TransactionRequest,
    TransactionScriptInputPair,
    TransactionScriptInputPairArray,
    WebClient
} = await wasm({
    importHook: () => {
        return new URL("assets/miden_client_web.wasm", import.meta.url); // the name before .wasm needs to match the package name in Cargo.toml
    },
});

export {
    AccountId,
    AdviceMap,
    AuthSecretKey,
    Felt,
    FeltArray,
    FungibleAsset,
    Note,
    NoteAssets,
    NoteExecutionHint,
    NoteExecutionMode,
    NoteId,
    NoteIdAndArgs,
    NoteIdAndArgsArray,
    NoteInputs,
    NoteMetadata,
    NoteRecipient,
    NoteTag,
    NoteType,
    OutputNote,
    OutputNotesArray,
    Rpo256,
    TransactionRequest,
    TransactionScriptInputPair,
    TransactionScriptInputPairArray,
    WebClient
};
