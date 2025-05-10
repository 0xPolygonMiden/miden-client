import { Page } from "puppeteer";
import {
  Account,
  AccountBuilder,
  AccountComponent,
  AccountHeader,
  AccountId,
  AccountIdAnchor,
  AccountStorageMode,
  AccountType,
  AdviceMap,
  Assembler,
  AssemblerUtils,
  AuthSecretKey,
  ConsumableNoteRecord,
  Felt,
  FeltArray,
  FungibleAsset,
  Library,
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
  StorageMap,
  StorageSlot,
  TestUtils,
  TransactionFilter,
  TransactionKernel,
  TransactionProver,
  TransactionRequest,
  TransactionResult,
  TransactionRequestBuilder,
  TransactionScript,
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
    AccountBuilder: typeof AccountBuilder;
    AccountComponent: typeof AccountComponent;
    AccountHeader: typeof AccountHeader;
    AccountId: typeof AccountId;
    AccountIdAnchor: typeof AccountIdAnchor;
    AccountType: typeof AccountType;
    AccountStorageMode: typeof AccountStorageMode;
    AdviceMap: typeof AdviceMap;
    Assembler: typeof Assembler;
    AssemblerUtils: typeof AssemblerUtils;
    AuthSecretKey: typeof AuthSecretKey;
    ConsumableNoteRecord: typeof ConsumableNoteRecord;
    Felt: typeof Felt;
    FeltArray: typeof FeltArray;
    FungibleAsset: typeof FungibleAsset;
    Library: typeof Library;
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
    StorageMap: typeof StorageMap;
    StorageSlot: typeof StorageSlot;
    TestUtils: typeof TestUtils;
    TransactionFilter: typeof TransactionFilter;
    TransactionKernel: typeof TransactionKernel;
    TransactionProver: typeof TransactionProver;
    TransactionRequest: typeof TransactionRequest;
    TransactionResult: typeof TransactionResult;
    TransactionRequestBuilder: typeof TransactionRequestBuilder;
    TransactionScript: typeof TransactionScript;
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
