import Dexie from "dexie";
import { IAccount, IAccountAuth, IAccountCode, IAccountStorage, IAccountVault, ITransaction, ITransactionScript, IInputNote, IOutputNote, IStateSync, IBlockHeader, IChainMmrNode } from "./dbTypes";

export enum Table {
  AccountCode = 'accountCode',
  AccountStorage = 'accountStorage',
  AccountVaults = 'accountVaults',
  AccountAuth = 'accountAuth',
  Accounts = 'accounts',
  Transactions = 'transactions',
  TransactionScripts = 'transactionScripts',
  InputNotes = 'inputNotes',
  OutputNotes = 'outputNotes',
  StateSync = 'stateSync',
  BlockHeaders = 'blockHeaders',
  ChainMmrNodes = 'chainMmrNodes',
}

export const db = new Dexie('MidenClientDB')
db.version(1).stores({
  [Table.AccountCode]: indexes('root'),
  [Table.AccountStorage]: indexes('root'),
  [Table.AccountVaults]: indexes('root'),
  [Table.AccountAuth]: indexes('accountId'),
  [Table.Accounts]: indexes('[id+nonce]', 'id', 'committed', 'codeRoot', 'storageRoot', 'vaultRoot'),
  [Table.Transactions]: indexes('id', 'scriptHash', 'blockNum', 'commitHeight'),
  [Table.TransactionScripts]: indexes('scriptHash'),
  [Table.InputNotes]: indexes('noteId', 'recipient', 'status'),
  [Table.OutputNotes]: indexes('noteId', 'recipient', 'status'),
  [Table.StateSync]: indexes('blockNum'),
  [Table.BlockHeaders]: indexes('blockNum', 'hasClientNotes'),
  [Table.ChainMmrNodes]: indexes('id')
});

function indexes(...items: string[]) {
  return items.join(',');
}

export const accountCodes = db.table<IAccountCode, Blob>(Table.AccountCode);
export const accountStorages = db.table<IAccountStorage, Blob>(Table.AccountStorage);
export const accountVaults = db.table<IAccountVault, Blob>(Table.AccountVaults);
export const accountAuths = db.table<IAccountAuth, bigint>(Table.AccountAuth);
export const accounts = db.table<IAccount, bigint>(Table.Accounts);
export const transactions = db.table<ITransaction, Blob>(Table.Transactions);
export const transactionScripts = db.table<ITransactionScript, Blob>(Table.TransactionScripts);
export const inputNotes = db.table<IInputNote, Blob>(Table.InputNotes);
export const outputNotes = db.table<IOutputNote, Blob>(Table.OutputNotes);
export const stateSync = db.table<IStateSync, bigint>(Table.StateSync);
export const blockHeaders = db.table<IBlockHeader, bigint>(Table.BlockHeaders);
export const chainMmrNodes = db.table<IChainMmrNode, bigint>(Table.ChainMmrNodes);
