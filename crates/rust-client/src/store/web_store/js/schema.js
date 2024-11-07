import Dexie from "dexie";

const DATABASE_NAME = 'MidenClientDB';

export async function openDatabase() {
  console.log('Opening database...')
  try {
      await db.open();
      console.log("Database opened successfully");
      return true;
  } catch (err) {
      console.error("Failed to open database: ", err);
      return false;
  }
}

const Table = {
  AccountCode: 'accountCode',
  AccountStorage: 'accountStorage',
  AccountVaults: 'accountVaults',
  AccountAuth: 'accountAuth',
  Accounts: 'accounts',
  Transactions: 'transactions',
  TransactionScripts: 'transactionScripts',
  InputNotes: 'inputNotes',
  OutputNotes: 'outputNotes',
  NotesScripts: 'notesScripts',
  StateSync: 'stateSync',
  BlockHeaders: 'blockHeaders',
  ChainMmrNodes: 'chainMmrNodes',
};

const db = new Dexie(DATABASE_NAME);
db.version(1).stores({
  [Table.AccountCode]: indexes('root'),
  [Table.AccountStorage]: indexes('root'),
  [Table.AccountVaults]: indexes('root'),
  [Table.AccountAuth]: indexes('accountId', 'pubKey'),
  [Table.Accounts]: indexes('[id+nonce]', 'codeRoot', 'storageRoot', 'vaultRoot', 'accountHash'),
  [Table.Transactions]: indexes('id'),
  [Table.TransactionScripts]: indexes('scriptHash'),
  [Table.InputNotes]: indexes('noteId', 'recipient', 'status', 'importedTag', 'ignored', 'nullifier'),
  [Table.OutputNotes]: indexes('noteId', 'recipient', 'status', 'importedTag', 'ignored', 'nullifier'),
  [Table.NotesScripts]: indexes('scriptHash'),
  [Table.StateSync]: indexes('id'),
  [Table.BlockHeaders]: indexes('blockNum', 'hasClientNotes'),
  [Table.ChainMmrNodes]: indexes('id'),
});

function indexes(...items) {
  return items.join(',');
}

db.on('populate', () => {
  // Populate the stateSync table with default values
  db.stateSync.put({ id: 1, blockNum: "0", tags: null });
});

const accountCodes = db.table(Table.AccountCode);
const accountStorages = db.table(Table.AccountStorage);
const accountVaults = db.table(Table.AccountVaults);
const accountAuths = db.table(Table.AccountAuth);
const accounts = db.table(Table.Accounts);
const transactions = db.table(Table.Transactions);
const transactionScripts = db.table(Table.TransactionScripts);
const inputNotes = db.table(Table.InputNotes);
const outputNotes = db.table(Table.OutputNotes);
const notesScripts = db.table(Table.NotesScripts);
const stateSync = db.table(Table.StateSync);
const blockHeaders = db.table(Table.BlockHeaders);
const chainMmrNodes = db.table(Table.ChainMmrNodes);

export { 
    db,
    accountCodes, 
    accountStorages, 
    accountVaults, 
    accountAuths, 
    accounts, 
    transactions,
    transactionScripts,
    inputNotes,
    outputNotes,
    notesScripts,
    stateSync,
    blockHeaders,
    chainMmrNodes,
};
