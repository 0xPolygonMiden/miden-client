// import { WebClient as WasmWebClient } from "./crates/miden_client_web";

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
  NewSwapTransactionResult,
  NewTransactionResult,
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
  NoteScript,
  NoteTag,
  NoteType,
  OutputNote,
  OutputNotesArray,
  Rpo256,
  SerializedAccountHeader,
  TestUtils,
  TransactionFilter,
  TransactionProver,
  TransactionRequest,
  TransactionRequestBuilder,
  TransactionScriptInputPair,
  TransactionScriptInputPairArray,
  Word,
  WebClient
} from "./crates/miden_client_web";

// // Define WebClient args
// export type SerializedAccountStorageMode = "private" | "public";
// export type SerializedNoteType = "private" | "public" | "encrypted";

// // Extend WASM WebClient but override methods that use workers
// export class WebClient extends WasmWebClient {
//   constructor(...args: any[]);
//   // new_wallet(storageMode: SerializedAccountStorageMode, mutable: boolean): Promise<Account>;
//   new_faucet(storageMode: SerializedAccountStorageMode, nonFungible: boolean, tokenSymbol: string, decimals: number, maxSupply: string): Promise<Account>;
//   // new_mint_transaction(target_account_id: string, faucet_id: string, note_type: SerializedNoteType, amount: string): Promise<string>;
//   terminate(): void;
// }
