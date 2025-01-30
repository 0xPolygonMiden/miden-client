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
  faucetAccountId: string,
  sync: boolean = true
): Promise<MintTransactionResult> => {
  return await testingPage.evaluate(
    async (_targetAccountId, _faucetAccountId, _sync) => {
      const client = window.client;

      console.log("TEST: about to call new_mint_transaction");
      console.log("TEST: targetAccountId", JSON.stringify(_targetAccountId));
      console.log("TEST: faucetAccountId", JSON.stringify(_faucetAccountId));

      const targetAccountId = window.AccountId.from_hex(_targetAccountId);
      const faucetAccountId = window.AccountId.from_hex(_faucetAccountId);

      await client.fetch_and_cache_account_auth_by_pub_key(faucetAccountId);
      console.log("TEST: fetch_and_cache_account_auth_by_pub_key finished");
      const new_mint_transaction_result = await client.new_mint_transaction(
        targetAccountId,
        faucetAccountId,
        window.NoteType.private(),
        BigInt(1000)
      );
      console.log("TEST: new_mint_transaction finished");
      console.log("TEST: new_mint_transaction_result", JSON.stringify(new_mint_transaction_result, null, 2));

      if (_sync) {
        await window.helpers.waitForTransaction(
          new_mint_transaction_result.transactionId
          // TODO: Add Back
          // new_mint_transaction_result.executed_transaction().id().to_hex()
        );
      }

      // TODO: Add Back
      // return {
      //   transactionId: new_mint_transaction_result
      //     .executed_transaction()
      //     .id()
      //     .to_hex(),
      //   numOutputNotesCreated: new_mint_transaction_result
      //     .created_notes()
      //     .num_notes(),
      //   nonce: new_mint_transaction_result.account_delta().nonce()?.to_string(),
      //   createdNoteId: new_mint_transaction_result
      //     .created_notes()
      //     .notes()[0]
      //     .id()
      //     .to_string(),
      // };

      return new_mint_transaction_result;
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

      await client.fetch_and_cache_account_auth_by_pub_key(
        window.AccountId.from_hex(_faucetAccountId)
      );
      let mint_transaction_result = await client.new_mint_transaction(
        senderAccountId,
        window.AccountId.from_hex(_faucetAccountId),
        window.NoteType.private(),
        BigInt(_amount)
      );
      // TODO: Add Back
      // let created_notes = mint_transaction_result.created_notes().notes();
      // let created_note_ids = created_notes.map((note) => note.id().to_string());
      await window.helpers.waitForTransaction(
        mint_transaction_result.transactionId
        // TODO: Add Back
        // mint_transaction_result.executed_transaction().id().to_hex()
      );

      await client.fetch_and_cache_account_auth_by_pub_key(senderAccountId);
      const consume_transaction_result = await client.new_consume_transaction(
        senderAccountId,
        [mint_transaction_result.createdNoteId]
        // TODO: Add Back
        //created_note_ids
      );
      await window.helpers.waitForTransaction(
        consume_transaction_result.transactionId
        // TODO: Add Back
        // consume_transaction_result.executed_transaction().id().to_hex()
      );

      await client.fetch_and_cache_account_auth_by_pub_key(senderAccountId);
      let send_transaction_result = await client.new_send_transaction(
        senderAccountId,
        targetAccountId,
        faucetAccountId,
        window.NoteType.public(),
        BigInt(_amount),
        _recallHeight
      );
      // TODO: Add Back
      // let send_created_notes = send_transaction_result.created_notes().notes();
      // let send_created_note_ids = send_created_notes.map((note) =>
      //   note.id().to_string()
      // );

      await window.helpers.waitForTransaction(
        send_transaction_result.transactionId
        // TODO: Add Back
        // send_transaction_result.executed_transaction().id().to_hex()
      );

      // TODO: Add Back
      // return send_created_note_ids;
      return send_transaction_result.noteIds;
    },
    senderAccountId,
    targetAccountId,
    faucetAccountId,
    amount,
    recallHeight
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
      console.log("TEST: consumedTransaction called");
      console.log("TEST: targetAccountId", JSON.stringify(_targetAccountId));
      console.log("TEST: faucetId", JSON.stringify(_faucetId));

      const targetAccountId = window.AccountId.from_hex(_targetAccountId);
      const faucetId = window.AccountId.from_hex(_faucetId);

      await client.fetch_and_cache_account_auth_by_pub_key(targetAccountId);
      const consumeTransactionResult = await client.new_consume_transaction(
        targetAccountId,
        [_noteId]
      );
      console.log("TEST: consumeTransactionResult", JSON.stringify(consumeTransactionResult, null, 2));
      await window.helpers.waitForTransaction(
        consumeTransactionResult.transactionId
        // TODO: Add Back
        // consumeTransactionResult.executed_transaction().id().to_hex()
      );

      const changedTargetAccount = await client.get_account(targetAccountId);

      // TODO: Add Back
      // return {
      //   transactionId: consumeTransactionResult
      //     .executed_transaction()
      //     .id()
      //     .to_hex(),
      //   nonce: consumeTransactionResult.account_delta().nonce()?.to_string(),
      //   numConsumedNotes: consumeTransactionResult.consumed_notes().num_notes(),
      //   targetAccountBalanace: changedTargetAccount
      //     .vault()
      //     .get_balance(faucetId)
      //     .toString(),
      // };
      return {
        ...consumeTransactionResult,
        targetAccountBalanace: changedTargetAccount
          .vault()
          .get_balance(faucetId)
          .toString(),
      }
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
      console.log("TEST: about to call new_wallet");
      const account = await client.new_wallet(
        window.AccountStorageMode.private(),
        true
      );
      console.log("TEST: new_wallet finished");
      console.log("TEST: account id string", JSON.stringify(account.id().to_string(), null, 2));
      console.log("TEST: about to call new_faucet");
      const faucetAccount = await client.new_faucet(
        window.AccountStorageMode.private(),
        false,
        "DAG",
        8,
        BigInt(10000000)
      );
      console.log("TEST: new_faucet finished");
      console.log("TEST: faucet account id string", JSON.stringify(faucetAccount.id().to_string(), null, 2));
      await client.sync_state();
      console.log("TEST: sync_state finished");

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
    const client = window.client;
    const summary = await client.sync_state();
    return {
      blockNum: summary.block_num(),
    };
  });
};

// Misc test utils

export const isValidAddress = (address: string) => {
  expect(address.startsWith("0x")).to.be.true;
};

// Constants

export const badHexId =
  "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
