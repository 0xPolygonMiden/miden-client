import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import {
  consumeTransaction,
  mintTransaction,
  setupWalletAndFaucet,
} from "./webClientTestUtils";

// GET_TRANSACTIONS TESTS
// =======================================================================================================

interface GetAllTransactionsResult {
  transactionIds: string[];
  uncomittedTransactionIds: string[];
}

const getAllTransactions = async (): Promise<GetAllTransactionsResult> => {
  return await testingPage.evaluate(async () => {
    const client = window.client;

    let transactions = await client.get_transactions(
      window.TransactionFilter.all()
    );
    let uncomittedTransactions = await client.get_transactions(
      window.TransactionFilter.uncomitted()
    );
    let transactionIds = transactions.map((transaction) =>
      transaction.id().to_hex()
    );
    let uncomittedTransactionIds = uncomittedTransactions.map((transaction) =>
      transaction.id().to_hex()
    );

    return {
      transactionIds: transactionIds,
      uncomittedTransactionIds: uncomittedTransactionIds,
    };
  });
};

describe("get_transactions tests", () => {
  it("get_transactions retrieves all transactions successfully", async () => {
    const { accountId, faucetId } = await setupWalletAndFaucet();
    const { transactionId: mintTransactionId, createdNoteId } =
      await mintTransaction(accountId, faucetId);
    const { transactionId: consumeTransactionId } = await consumeTransaction(
      accountId,
      faucetId,
      createdNoteId
    );

    const result = await getAllTransactions();

    expect(result.transactionIds).to.include(mintTransactionId);
    expect(result.transactionIds).to.include(consumeTransactionId);
    expect(result.uncomittedTransactionIds.length).to.equal(0);
  });

  it("get_transactions retrieves uncommitted transactions successfully", async () => {
    const { accountId, faucetId } = await setupWalletAndFaucet();
    const { transactionId: mintTransactionId } = await mintTransaction(
      accountId,
      faucetId,
      false
    );

    const result = await getAllTransactions();

    expect(result.transactionIds).to.include(mintTransactionId);
    expect(result.uncomittedTransactionIds).to.include(mintTransactionId);
    expect(result.transactionIds.length).to.equal(
      result.uncomittedTransactionIds.length
    );
  });

  it("get_transactions retrieves no transactions successfully", async () => {
    const result = await getAllTransactions();

    expect(result.transactionIds.length).to.equal(0);
    expect(result.uncomittedTransactionIds.length).to.equal(0);
  });
});

// COMPILE_TX_SCRIPT TESTS
// =======================================================================================================

interface CompileTxScriptResult {
  scriptHash: string;
}

export const compileTxScript = async (
  script: string
): Promise<CompileTxScriptResult> => {
  return await testingPage.evaluate(async (_script) => {
    const client = window.client;

    let walletAccount = await client.new_wallet(
      window.AccountStorageMode.private(),
      true
    );

    let account_auth = await client.get_account_auth(walletAccount.id());
    let public_key = account_auth.get_rpo_falcon_512_public_key_as_word();
    let secret_key = account_auth.get_rpo_falcon_512_secret_key_as_felts();
    let transcription_script_input_pair_array =
      new window.TransactionScriptInputPairArray([
        new window.TransactionScriptInputPair(public_key, secret_key),
      ]);

    const compiledScript = await client.compile_tx_script(
      _script,
      transcription_script_input_pair_array
    );

    return {
      scriptHash: compiledScript.hash().to_hex(),
    };
  }, script);
};

describe("compile_tx_script tests", () => {
  it("compile_tx_script compiles script successfully", async () => {
    const script = `
            use.miden::contracts::auth::basic->auth_tx
            use.miden::kernels::tx::prologue
            use.miden::kernels::tx::memory

            begin
                push.0 push.0
                # => [0, 0]
                assert_eq

                call.auth_tx::auth_tx_rpo_falcon512
            end
        `;
    const result = await compileTxScript(script);

    expect(result.scriptHash).to.not.be.empty;
  });

  it("compile_tx_script does not compile script successfully", async () => {
    const script = "fakeScript";

    await expect(compileTxScript(script)).to.be.rejectedWith(
      `Failed to compile transaction script: Transaction script error: AssemblyError("invalid syntax")`
    );
  });
});
