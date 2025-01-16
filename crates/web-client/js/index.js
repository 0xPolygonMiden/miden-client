import wasm from "../dist/wasm.js";

const {
  Account,
  AccountHeader,
  AccountId,
  AccountStorageMode,
  AdviceMap,
  AuthSecretKey,
  ConsumableNoteRecord,
  Felt,
  FeltArray,
  FungibleAsset,
  InputNoteState,
  Note,
  NoteAssets,
  NoteConsumability,
  NoteExecutionHint,
  NoteExecutionMode,
  NoteFilter,
  NoteFilterTypes,
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
  ProverWrapper,
  Rpo256,
  TestUtils,
  TransactionFilter,
  TransactionRequest,
  TransactionRequestBuilder,
  TransactionScriptInputPair,
  TransactionScriptInputPairArray,
  WebClient,
} = await wasm({
  importHook: () => {
    return new URL("assets/miden_client_web.wasm", import.meta.url); // the name before .wasm needs to match the package name in Cargo.toml
  },
});

export {
  Account,
  AccountHeader,
  AccountId,
  AccountStorageMode,
  AdviceMap,
  AuthSecretKey,
  ConsumableNoteRecord,
  Felt,
  FeltArray,
  FungibleAsset,
  InputNoteState,
  Note,
  NoteAssets,
  NoteConsumability,
  NoteExecutionHint,
  NoteExecutionMode,
  NoteFilter,
  NoteFilterTypes,
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
  ProverWrapper,
  Rpo256,
  TestUtils,
  TransactionFilter,
  TransactionRequest,
  TransactionRequestBuilder,
  TransactionScriptInputPair,
  TransactionScriptInputPairArray,
  WebClient,
};
