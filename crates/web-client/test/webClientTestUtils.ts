import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";

interface MintTransactionResult {
  transactionId: string;
  numOutputNotesCreated: number;
  nonce: string | undefined;
  createdNoteId: string;
}

export enum StorageMode {
  PRIVATE = "private",
  PUBLIC = "public",
}

// SDK functions

export const mintTransaction = async (
  targetAccountId: string,
  faucetAccountId: string,
  sync: boolean = true
): Promise<MintTransactionResult> => {
  return await testingPage.evaluate(
    async (_targetAccountId, _faucetAccountId, _sync) => {
      const client = window.client;

      const targetAccountId = window.AccountId.fromHex(_targetAccountId);
      const faucetAccountId = window.AccountId.fromHex(_faucetAccountId);

      const newMintTransactionResult = await client.newMintTransaction(
        targetAccountId,
        faucetAccountId,
        window.NoteType.private(),
        BigInt(1000)
      );

      if (_sync) {
        await window.helpers.waitForTransaction(
          newMintTransactionResult.executedTransaction().id().toHex()
        );
      }

      return {
        transactionId: newMintTransactionResult
          .executedTransaction()
          .id()
          .toHex(),
        numOutputNotesCreated: newMintTransactionResult
          .createdNotes()
          .numNotes(),
        nonce: newMintTransactionResult.accountDelta().nonce()?.toString(),
        createdNoteId: newMintTransactionResult
          .createdNotes()
          .notes()[0]
          .id()
          .toString(),
      };
    },
    targetAccountId,
    faucetAccountId,
    sync
  );
};

export const sendTransaction = async (
  senderAccountId: string,
  targetAccountId: string,
  faucetAccountId: string,
  amount: number,
  recallHeight?: number
) => {
  return testingPage.evaluate(
    async (
      _senderAccountId,
      _targetAccountId,
      _faucetAccountId,
      _amount,
      _recallHeight
    ) => {
      const client = window.client;

      const senderAccountId = window.AccountId.fromHex(_senderAccountId);
      const targetAccountId = window.AccountId.fromHex(_targetAccountId);
      const faucetAccountId = window.AccountId.fromHex(_faucetAccountId);

      let mintTransactionResult = await client.newMintTransaction(
        senderAccountId,
        window.AccountId.fromHex(_faucetAccountId),
        window.NoteType.private(),
        BigInt(_amount)
      );
      let createdNotes = mintTransactionResult.createdNotes().notes();
      let createdNoteIds = createdNotes.map((note) => note.id().toString());
      await window.helpers.waitForTransaction(
        mintTransactionResult.executedTransaction().id().toHex()
      );

      const consumeTransactionResult = await client.newConsumeTransaction(
        senderAccountId,
        createdNoteIds
      );
      await window.helpers.waitForTransaction(
        consumeTransactionResult.executedTransaction().id().toHex()
      );

      let sendTransactionResult = await client.newSendTransaction(
        senderAccountId,
        targetAccountId,
        faucetAccountId,
        window.NoteType.public(),
        BigInt(_amount),
        _recallHeight
      );
      let sendCreatedNotes = sendTransactionResult.createdNotes().notes();
      let sendCreatedNoteIds = sendCreatedNotes.map((note) =>
        note.id().toString()
      );

      await window.helpers.waitForTransaction(
        sendTransactionResult.executedTransaction().id().toHex()
      );

      return sendCreatedNoteIds;
    },
    senderAccountId,
    targetAccountId,
    faucetAccountId,
    amount,
    recallHeight
  );
};

export interface NewAccountTestResult {
  id: string;
  nonce: string;
  vaultCommitment: string;
  storageCommitment: string;
  codeCommitment: string;
  isFaucet: boolean;
  isRegularAccount: boolean;
  isUpdatable: boolean;
  isPublic: boolean;
  isNew: boolean;
}
export const createNewWallet = async ({
  storageMode,
  mutable,
  clientSeed,
  isolatedClient,
  walletSeed,
}: {
  storageMode: StorageMode;
  mutable: boolean;
  clientSeed?: Uint8Array;
  isolatedClient?: boolean;
  walletSeed?: Uint8Array;
}): Promise<NewAccountTestResult> => {
  // Serialize initSeed for Puppeteer
  const serializedClientSeed = clientSeed ? Array.from(clientSeed) : null;
  const serializedWalletSeed = walletSeed ? Array.from(walletSeed) : null;

  return await testingPage.evaluate(
    async (
      _storageMode,
      _mutable,
      _serializedClientSeed,
      _isolatedClient,
      _serializedWalletSeed
    ) => {
      if (_isolatedClient) {
        // Reconstruct Uint8Array inside the browser context
        const _clientSeed = _serializedClientSeed
          ? new Uint8Array(_serializedClientSeed)
          : undefined;

        await window.helpers.refreshClient(_clientSeed);
      }

      let _walletSeed;
      if (_serializedWalletSeed) {
        _walletSeed = new Uint8Array(_serializedWalletSeed);
      }

      let client = window.client;
      const accountStorageMode =
        _storageMode === "private"
          ? window.AccountStorageMode.private()
          : window.AccountStorageMode.public();

      const newWallet = await client.newWallet(
        accountStorageMode,
        _mutable,
        _walletSeed
      );

      return {
        id: newWallet.id().toString(),
        nonce: newWallet.nonce().toString(),
        vaultCommitment: newWallet.vault().commitment().toHex(),
        storageCommitment: newWallet.storage().commitment().toHex(),
        codeCommitment: newWallet.code().commitment().toHex(),
        isFaucet: newWallet.isFaucet(),
        isRegularAccount: newWallet.isRegularAccount(),
        isUpdatable: newWallet.isUpdatable(),
        isPublic: newWallet.isPublic(),
        isNew: newWallet.isNew(),
      };
    },
    storageMode,
    mutable,
    serializedClientSeed,
    isolatedClient,
    serializedWalletSeed
  );
};

export const createNewFaucet = async (
  storageMode: StorageMode = StorageMode.PUBLIC,
  nonFungible: boolean = false,
  tokenSymbol: string = "DAG",
  decimals: number = 8,
  maxSupply: bigint = BigInt(10000000)
): Promise<NewAccountTestResult> => {
  return await testingPage.evaluate(
    async (_storageMode, _nonFungible, _tokenSymbol, _decimals, _maxSupply) => {
      const client = window.client;
      const accountStorageMode =
        _storageMode === "private"
          ? window.AccountStorageMode.private()
          : window.AccountStorageMode.public();
      const newFaucet = await client.newFaucet(
        accountStorageMode,
        _nonFungible,
        _tokenSymbol,
        _decimals,
        _maxSupply
      );
      return {
        id: newFaucet.id().toString(),
        nonce: newFaucet.nonce().toString(),
        vaultCommitment: newFaucet.vault().commitment().toHex(),
        storageCommitment: newFaucet.storage().commitment().toHex(),
        codeCommitment: newFaucet.code().commitment().toHex(),
        isFaucet: newFaucet.isFaucet(),
        isRegularAccount: newFaucet.isRegularAccount(),
        isUpdatable: newFaucet.isUpdatable(),
        isPublic: newFaucet.isPublic(),
        isNew: newFaucet.isNew(),
      };
    },
    storageMode,
    nonFungible,
    tokenSymbol,
    decimals,
    maxSupply
  );
};

export const fundAccountFromFaucet = async (
  accountId: string,
  faucetId: string
) => {
  const mintResult = await mintTransaction(accountId, faucetId);
  return await consumeTransaction(
    accountId,
    faucetId,
    mintResult.createdNoteId
  );
};

export const getAccountBalance = async (
  accountId: string,
  faucetId: string
) => {
  return await testingPage.evaluate(
    async (_accountId, _faucetId) => {
      const client = window.client;
      const account = await client.getAccount(
        window.AccountId.fromHex(_accountId)
      );
      let balance = BigInt(0);
      if (account) {
        balance = account
          .vault()
          .getBalance(window.AccountId.fromHex(_faucetId));
      }
      return balance;
    },
    accountId,
    faucetId
  );
};

interface ConsumeTransactionResult {
  transactionId: string;
  nonce: string | undefined;
  numConsumedNotes: number;
  targetAccountBalanace: string;
}

export const consumeTransaction = async (
  targetAccountId: string,
  faucetId: string,
  noteId: string
): Promise<ConsumeTransactionResult> => {
  return await testingPage.evaluate(
    async (_targetAccountId, _faucetId, _noteId) => {
      const client = window.client;

      const targetAccountId = window.AccountId.fromHex(_targetAccountId);
      const faucetId = window.AccountId.fromHex(_faucetId);

      const consumeTransactionResult = await client.newConsumeTransaction(
        targetAccountId,
        [_noteId]
      );
      await window.helpers.waitForTransaction(
        consumeTransactionResult.executedTransaction().id().toHex()
      );

      const changedTargetAccount = await client.getAccount(targetAccountId);

      return {
        transactionId: consumeTransactionResult
          .executedTransaction()
          .id()
          .toHex(),
        nonce: consumeTransactionResult.accountDelta().nonce()?.toString(),
        numConsumedNotes: consumeTransactionResult.consumedNotes().numNotes(),
        targetAccountBalanace: changedTargetAccount
          .vault()
          .getBalance(faucetId)
          .toString(),
      };
    },
    targetAccountId,
    faucetId,
    noteId
  );
};

interface SetupWalletFaucetResult {
  accountId: string;
  faucetId: string;
  accountHash: string;
}

export const setupWalletAndFaucet =
  async (): Promise<SetupWalletFaucetResult> => {
    return await testingPage.evaluate(async () => {
      const client = window.client;
      const account = await client.newWallet(
        window.AccountStorageMode.private(),
        true
      );
      const faucetAccount = await client.newFaucet(
        window.AccountStorageMode.private(),
        false,
        "DAG",
        8,
        BigInt(10000000)
      );
      await client.syncState();

      return {
        accountId: account.id().toString(),
        accountHash: account.hash().toHex(),
        faucetId: faucetAccount.id().toString(),
      };
    });
  };

export const getAccount = async (accountId: string) => {
  return await testingPage.evaluate(async (_accountId) => {
    const client = window.client;
    const accountId = window.AccountId.fromHex(_accountId);
    const account = await client.getAccount(accountId);
    return {
      id: account?.id().toString(),
      hash: account?.hash().toHex(),
      nonce: account?.nonce().toString(),
      vaultCommitment: account?.vault().commitment().toHex(),
      storageCommitment: account?.storage().commitment().toHex(),
      codeCommitment: account?.code().commitment().toHex(),
    };
  }, accountId);
};

export const syncState = async () => {
  return await testingPage.evaluate(async () => {
    const client = window.client;
    const summary = await client.syncState();
    return {
      blockNum: summary.blockNum(),
    };
  });
};
export const clearStore = async () => {
  await testingPage.evaluate(async () => {
    // Open a connection to the list of databases
    const databases = await indexedDB.databases();
    for (const db of databases) {
      // Delete each database by name
      if (db.name) {
        indexedDB.deleteDatabase(db.name);
      }
    }
  });
};

// Misc test utils

export const isValidAddress = (address: string) => {
  expect(address.startsWith("0x")).to.be.true;
};

// Constants

export const badHexId =
  "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
