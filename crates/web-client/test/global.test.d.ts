import { Page } from "puppeteer";
import {
  Account,
  AccountHeader,
  AccountId,
  AccountStorageMode,
  AdviceMap,
  AuthSecretKey,
  Felt,
  FeltArray,
  FungibleAsset,
  Note,
  NoteAssets,
  NoteExecutionHint,
  NoteExecutionMode,
  NoteFilter,
  NoteFilterTypes,
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
  TestUtils,
  TransactionFilter,
  TransactionRequest,
  TransactionScriptInputPair,
  TransactionScriptInputPairArray,
  WebClient,
} from "../dist/index";

declare global {
  interface Window {
    client: WebClient;
    Account: typeof Account;
    AccountHeader: typeof AccountHeader;
    AccountId: typeof AccountId;
    AccountStorageMode: typeof AccountStorageMode;
    AdviceMap: typeof AdviceMap;
    AuthSecretKey: typeof AuthSecretKey;
    Felt: typeof Felt;
    FeltArray: typeof FeltArray;
    FungibleAsset: typeof FungibleAsset;
    Note: typeof Note;
    NoteAssets: typeof NoteAssets;
    NoteExecutionHint: typeof NoteExecutionHint;
    NoteExecutionMode: typeof NoteExecutionMode;
    NoteFilter: typeof NoteFilter;
    NoteFilterTypes: typeof NoteFilterTypes;
    NoteIdAndArgs: typeof NoteIdAndArgs;
    NoteIdAndArgsArray: typeof NoteIdAndArgsArray;
    NoteInputs: typeof NoteInputs;
    NoteMetadata: typeof NoteMetadata;
    NoteRecipient: typeof NoteRecipient;
    NoteTag: typeof NoteTag;
    NoteType: typeof NoteType;
    OutputNote: typeof OutputNote;
    OutputNotesArray: typeof OutputNotesArray;
    Rpo256: typeof Rpo256;
    TestUtils: typeof TestUtils;
    TransactionFilter: typeof TransactionFilter;
    TransactionRequest: typeof TransactionRequest;
    TransactionScriptInputPair: typeof TransactionScriptInputPair;
    TransactionScriptInputPairArray: typeof TransactionScriptInputPairArray;
    create_client: () => Promise<void>;
  }
}

declare module "./mocha.global.setup.mjs" {
  export const testingPage: Page;
}
