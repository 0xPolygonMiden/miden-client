import { Page } from "puppeteer";
import {
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
  Note,
  NoteAssets,
  NoteConsumability,
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
  TransactionProver,
  TransactionRequest,
  TransactionResult,
  TransactionRequestBuilder,
  TransactionScriptInputPair,
  TransactionScriptInputPairArray,
  Word,
  WebClient,
  NoteAndArgs,
  NoteAndArgsArray,
} from "../dist/index";

declare global {
  interface Window {
    client: WebClient;
    remoteProverUrl: string;
    remoteProverInstance: TransactionProver;
    Account: typeof Account;
    AccountHeader: typeof AccountHeader;
    AccountId: typeof AccountId;
    AccountStorageMode: typeof AccountStorageMode;
    AdviceMap: typeof AdviceMap;
    AuthSecretKey: typeof AuthSecretKey;
    ConsumableNoteRecord: typeof ConsumableNoteRecord;
    Felt: typeof Felt;
    FeltArray: typeof FeltArray;
    FungibleAsset: typeof FungibleAsset;
    Note: typeof Note;
    NoteAndArgs: typeof NoteAndArgs;
    NoteAndArgsArray: typeof NoteAndArgsArray;
    NoteAssets: typeof NoteAssets;
    NoteConsumability: typeof NoteConsumability;
    NoteExecutionHint: typeof NoteExecutionHint;
    NoteExecutionMode: typeof NoteExecutionMode;
    NoteFilter: typeof NoteFilter;
    NoteFilterTypes: typeof NoteFilterTypes;
    NoteIdAndArgs: typeof NoteIdAndArgs;
    NoteIdAndArgsArray: typeof NoteIdAndArgsArray;
    NoteInputs: typeof NoteInputs;
    NoteMetadata: typeof NoteMetadata;
    NoteRecipient: typeof NoteRecipient;
    NoteScript: typeof NoteScript;
    NoteTag: typeof NoteTag;
    NoteType: typeof NoteType;
    OutputNote: typeof OutputNote;
    OutputNotesArray: typeof OutputNotesArray;
    Rpo256: typeof Rpo256;
    TestUtils: typeof TestUtils;
    TransactionFilter: typeof TransactionFilter;
    TransactionProver: typeof TransactionProver;
    TransactionRequest: typeof TransactionRequest;
    TransactionResult: typeof TransactionResult;
    TransactionRequestBuilder: typeof TransactionRequestBuilder;
    TransactionScriptInputPair: typeof TransactionScriptInputPair;
    TransactionScriptInputPairArray: typeof TransactionScriptInputPairArray;
    WebClient: typeof WebClient;
    Word: typeof Word;
    createClient: () => Promise<void>;

    // Add the helpers namespace
    helpers: {
      waitForTransaction: (
        transactionId: string,
        maxWaitTime?: number,
        delayInterval?: number
      ) => Promise<void>;
      refreshClient: (initSeed?: Uint8Array) => Promise<void>;
    };
  }
}

declare module "./mocha.global.setup.mjs" {
  export const testingPage: Page;
}
