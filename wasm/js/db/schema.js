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
  InputNotes: 'inputNotes',
  OutputNotes: 'outputNotes',
  Greet: 'greets',
};

const db = new Dexie(DATABASE_NAME);
db.version(1).stores({
  [Table.AccountCode]: indexes('root'),
  [Table.AccountStorage]: indexes('root'),
  [Table.AccountVaults]: indexes('root'),
  [Table.AccountAuth]: indexes('accountId'),
  [Table.Accounts]: indexes('[id+nonce]', 'codeRoot', 'storageRoot', 'vaultRoot'),
  [Table.InputNotes]: indexes('noteId', 'recipient', 'status'),
  [Table.OutputNotes]: indexes('noteId', 'recipient', 'status'),
  [Table.Greet]: '++id',
});

function indexes(...items) {
  return items.join(',');
}

const accountCodes = db.table(Table.AccountCode);
const accountStorages = db.table(Table.AccountStorage);
const accountVaults = db.table(Table.AccountVaults);
const accountAuths = db.table(Table.AccountAuth);
const accounts = db.table(Table.Accounts);
const inputNotes = db.table(Table.InputNotes);
const outputNotes = db.table(Table.OutputNotes);
const greets = db.table(Table.Greet);

export { 
    accountCodes, 
    accountStorages, 
    accountVaults, 
    accountAuths, 
    accounts, 
    inputNotes,
    outputNotes,
    greets,
};
