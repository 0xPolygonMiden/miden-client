export interface IAccountCode {
  root: Blob;
  procedures: Blob;
  module: Blob;
}

export interface IAccountStorage {
  root: Blob;
  slots: Blob;
}

export interface IAccountVault {
  root: Blob;
  assets: Blob;
}

export interface IAccountAuth {
  accountId: bigint;
  authInfo: Blob;
}

export interface IAccount {
  id: bigint;
  codeRoot: Blob;
  storageRoot: Blob;
  vaultRoot: Blob;
  nonce: bigint;
  committed: boolean;
  accountSeed: Blob;
}

export interface ITransaction {
  id: Blob;
  accountId: bigint;
  initAccountState: Blob;
  finalAccountState: Blob;
  inputNotes: Blob;
  outputNotes: Blob;
  scriptHash: Blob;
  scriptInputs: Blob;
  blockNum: bigint;
  commitHeight: bigint;
}

export interface ITransactionScript {
  scriptHash: Blob;
  program: Blob;
}

export interface IInputNote {
  noteId: Blob;
  recipient: Blob;
  assets: Blob;
  status: string;
  inclusionProof: string;
  metadata: Blob;
  details: string;
}

export interface IOutputNote {
  noteId: Blob;
  recipient: Blob;
  assets: Blob;
  status: string;
  inclusionProof: string;
  metadata: Blob;
  details: string;
}

export interface IStateSync {
  blockNum: bigint;
  tags: Blob;
}

export interface IBlockHeader {
  blockNum: bigint;
  header: Blob;
  chainMmrPeaks: Blob;
  hasClientNotes: boolean;
}

export interface IChainMmrNode {
  id: bigint;
  node: Blob;
}