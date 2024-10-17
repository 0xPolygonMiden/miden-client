import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";

interface MintTransactionResult {
  transactionId: string;
  numOutputNotesCreated: number;
  nonce: string | undefined;
  createdNoteId: string;
}

// SDK functions

export const mintTransaction = async (
  targetAccountId: string,
  faucetAccountId: string
): Promise<MintTransactionResult> => {
  return await testingPage.evaluate(
    async (_targetAccountId, _faucetAccountId) => {
      if (!window.client) {
        await window.create_client();
      }

      const client = window.client;

      await new Promise((r) => setTimeout(r, 20000));
      const targetAccountId = window.AccountId.from_hex(_targetAccountId);
      const faucetAccountId = window.AccountId.from_hex(_faucetAccountId);

      await client.fetch_and_cache_account_auth_by_pub_key(faucetAccountId);
      const new_mint_transaction_result = await client.new_mint_transaction(
        targetAccountId,
        faucetAccountId,
        window.NoteType.private(),
        BigInt(1000)
      );

      await new Promise((r) => setTimeout(r, 20000)); // TODO: Replace this with loop of sync -> check uncommitted transactions -> sleep
      await client.sync_state();

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
    faucetAccountId
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

      await new Promise((r) => setTimeout(r, 20000)); // TODO: Replace this with loop of sync -> check uncommitted transactions -> sleep
      await client.sync_state();

      await client.fetch_and_cache_account_auth_by_pub_key(targetAccountId);
      const consumeTransactionResult = await client.new_consume_transaction(
        targetAccountId,
        [_noteId]
      );
      await new Promise((r) => setTimeout(r, 20000)); // TODO: Replace this with loop of sync -> check uncommitted transactions -> sleep
      await client.sync_state();

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

export const fetchAndCacheAccountAuth = async (accountId: string) => {
  return await testingPage.evaluate(async (_accountId) => {
    const accountId = window.AccountId.from_hex(_accountId);
    const client = window.client;
    await client.fetch_and_cache_account_auth_by_pub_key(accountId);
  }, accountId);
};

export const syncState = async () => {
  return await testingPage.evaluate(async () => {
    if (!window.client) {
      await window.create_client();
    }
    const client = window.client;
    await client.sync_state();
  });
};

// Misc test utils

export const isValidAddress = (address: string) => {
  expect(address.startsWith("0x")).to.be.true;
};

// Constants
export const badHexId =
  "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
