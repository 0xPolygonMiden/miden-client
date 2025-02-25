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

      const targetAccountId = window.AccountId.from_hex(_targetAccountId);
      const faucetAccountId = window.AccountId.from_hex(_faucetAccountId);

      const new_mint_transaction_result = await client.new_mint_transaction(
        targetAccountId,
        faucetAccountId,
        window.NoteType.private(),
        BigInt(1000)
      );

      if (_sync) {
        await window.helpers.waitForTransaction(
          new_mint_transaction_result.executed_transaction().id().to_hex()
        );
      }

      return {
        transactionId: new_mint_transaction_result
          .executed_transaction()
          .id()
          .to_hex(),
        numOutputNotesCreated: new_mint_transaction_result
          .created_notes()
          .num_notes(),
        nonce: new_mint_transaction_result.account_delta().nonce()?.to_string(),
        createdNoteId: new_mint_transaction_result
          .created_notes()
          .notes()[0]
          .id()
          .to_string(),
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

      const senderAccountId = window.AccountId.from_hex(_senderAccountId);
      const targetAccountId = window.AccountId.from_hex(_targetAccountId);
      const faucetAccountId = window.AccountId.from_hex(_faucetAccountId);

      let mint_transaction_result = await client.new_mint_transaction(
        senderAccountId,
        window.AccountId.from_hex(_faucetAccountId),
        window.NoteType.private(),
        BigInt(_amount)
      );
      let created_notes = mint_transaction_result.created_notes().notes();
      let created_note_ids = created_notes.map((note) => note.id().to_string());
      await window.helpers.waitForTransaction(
        mint_transaction_result.executed_transaction().id().to_hex()
      );

      const consume_transaction_result = await client.new_consume_transaction(
        senderAccountId,
        created_note_ids
      );
      await window.helpers.waitForTransaction(
        consume_transaction_result.executed_transaction().id().to_hex()
      );

      let send_transaction_result = await client.new_send_transaction(
        senderAccountId,
        targetAccountId,
        faucetAccountId,
        window.NoteType.public(),
        BigInt(_amount),
        _recallHeight
      );
      let send_created_notes = send_transaction_result.created_notes().notes();
      let send_created_note_ids = send_created_notes.map((note) =>
        note.id().to_string()
      );

      await window.helpers.waitForTransaction(
        send_transaction_result.executed_transaction().id().to_hex()
      );

      return send_created_note_ids;
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
  vault_commitment: string;
  storage_commitment: string;
  code_commitment: string;
  is_faucet: boolean;
  is_regular_account: boolean;
  is_updatable: boolean;
  is_public: boolean;
  is_new: boolean;
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

      const newWallet = await client.new_wallet(
        accountStorageMode,
        _mutable,
        _walletSeed
      );

      return {
        id: newWallet.id().to_string(),
        nonce: newWallet.nonce().to_string(),
        vault_commitment: newWallet.vault().commitment().to_hex(),
        storage_commitment: newWallet.storage().commitment().to_hex(),
        code_commitment: newWallet.code().commitment().to_hex(),
        is_faucet: newWallet.is_faucet(),
        is_regular_account: newWallet.is_regular_account(),
        is_updatable: newWallet.is_updatable(),
        is_public: newWallet.is_public(),
        is_new: newWallet.is_new(),
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
      const newFaucet = await client.new_faucet(
        accountStorageMode,
        _nonFungible,
        _tokenSymbol,
        _decimals,
        _maxSupply
      );
      return {
        id: newFaucet.id().to_string(),
        nonce: newFaucet.nonce().to_string(),
        vault_commitment: newFaucet.vault().commitment().to_hex(),
        storage_commitment: newFaucet.storage().commitment().to_hex(),
        code_commitment: newFaucet.code().commitment().to_hex(),
        is_faucet: newFaucet.is_faucet(),
        is_regular_account: newFaucet.is_regular_account(),
        is_updatable: newFaucet.is_updatable(),
        is_public: newFaucet.is_public(),
        is_new: newFaucet.is_new(),
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
      const account = await client.get_account(
        window.AccountId.from_hex(_accountId)
      );
      let balance = BigInt(0);
      if (account) {
        balance = account
          .vault()
          .get_balance(window.AccountId.from_hex(_faucetId));
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

      const targetAccountId = window.AccountId.from_hex(_targetAccountId);
      const faucetId = window.AccountId.from_hex(_faucetId);

      const consumeTransactionResult = await client.new_consume_transaction(
        targetAccountId,
        [_noteId]
      );
      await window.helpers.waitForTransaction(
        consumeTransactionResult.executed_transaction().id().to_hex()
      );

      const changedTargetAccount = await client.get_account(targetAccountId);

      return {
        transactionId: consumeTransactionResult
          .executed_transaction()
          .id()
          .to_hex(),
        nonce: consumeTransactionResult.account_delta().nonce()?.to_string(),
        numConsumedNotes: consumeTransactionResult.consumed_notes().num_notes(),
        targetAccountBalanace: changedTargetAccount
          .vault()
          .get_balance(faucetId)
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
}

export const setupWalletAndFaucet =
  async (): Promise<SetupWalletFaucetResult> => {
    return await testingPage.evaluate(async () => {
      const client = window.client;
      const account = await client.new_wallet(
        window.AccountStorageMode.private(),
        true
      );
      const faucetAccount = await client.new_faucet(
        window.AccountStorageMode.private(),
        false,
        "DAG",
        8,
        BigInt(10000000)
      );
      await client.sync_state();

      return {
        accountId: account.id().to_string(),
        faucetId: faucetAccount.id().to_string(),
      };
    });
  };

export const getAccount = async (accountId: string) => {
  return await testingPage.evaluate(async (_accountId) => {
    const client = window.client;
    const accountId = window.AccountId.from_hex(_accountId);
    const account = await client.get_account(accountId);
    return {
      id: account?.id().to_string(),
      hash: account?.hash().to_hex(),
      nonce: account?.nonce().to_string(),
      vaultCommitment: account?.vault().commitment().to_hex(),
      storageCommitment: account?.storage().commitment().to_hex(),
      codeCommitment: account?.code().commitment().to_hex(),
    };
  }, accountId);
};

export const syncState = async () => {
  return await testingPage.evaluate(async () => {
    const client = window.client;
    const summary = await client.sync_state();
    return {
      blockNum: summary.block_num(),
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
