import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import {
  consumeTransaction,
  mintAndConsumeTransaction,
  mintTransaction,
  setupWalletAndFaucet,
} from "./webClientTestUtils";

// GET_TRANSACTIONS TESTS
// =======================================================================================================

interface GetAllTransactionsResult {
  transactionIds: string[];
  uncommittedTransactionIds: string[];
}

const getAllTransactions = async (): Promise<GetAllTransactionsResult> => {
  return await testingPage.evaluate(async () => {
    const client = window.client;

    let transactions = await client.getTransactions(
      window.TransactionFilter.all()
    );
    let uncommittedTransactions = await client.getTransactions(
      window.TransactionFilter.uncommitted()
    );
    let transactionIds = transactions.map((transaction) =>
      transaction.id().toHex()
    );
    let uncommittedTransactionIds = uncommittedTransactions.map((transaction) =>
      transaction.id().toHex()
    );

    return {
      transactionIds: transactionIds,
      uncommittedTransactionIds: uncommittedTransactionIds,
    };
  });
};

describe("get_transactions tests", () => {
  it("get_transactions retrieves all transactions successfully", async () => {
    const { accountId, faucetId } = await setupWalletAndFaucet();

    const { mintResult, consumeResult } = await mintAndConsumeTransaction(
      accountId,
      faucetId
    );

    const result = await getAllTransactions();

    expect(result.transactionIds).to.include(mintResult.transactionId);
    expect(result.transactionIds).to.include(consumeResult.transactionId);
    expect(result.uncommittedTransactionIds.length).to.equal(0);
  });

  it("get_transactions retrieves uncommitted transactions successfully", async () => {
    const { accountId, faucetId } = await setupWalletAndFaucet();
    const { transactionId: mintTransactionId } = await mintTransaction(
      accountId,
      faucetId,
      false,
      false
    );

    const result = await getAllTransactions();

    expect(result.transactionIds).to.include(mintTransactionId);
    expect(result.uncommittedTransactionIds).to.include(mintTransactionId);
    expect(result.transactionIds.length).to.equal(
      result.uncommittedTransactionIds.length
    );
  });

  it("get_transactions retrieves no transactions successfully", async () => {
    const result = await getAllTransactions();

    expect(result.transactionIds.length).to.equal(0);
    expect(result.uncommittedTransactionIds.length).to.equal(0);
  });
});

// COMPILE_TX_SCRIPT TESTS
// =======================================================================================================

interface CompileTxScriptResult {
  scriptRoot: string;
}

export const compileTxScript = async (
  script: string
): Promise<CompileTxScriptResult> => {
  return await testingPage.evaluate(async (_script) => {
    const client = window.client;

    let walletAccount = await client.newWallet(
      window.AccountStorageMode.private(),
      true
    );

    const compiledScript = await client.compileTxScript(_script);

    return {
      scriptRoot: compiledScript.root().toHex(),
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

    expect(result.scriptRoot).to.not.be.empty;
  });

  it("compile_tx_script does not compile script successfully", async () => {
    const script = "fakeScript";

    await expect(compileTxScript(script)).to.be.rejectedWith(
      /failed to compile transaction script:/
    );
  });
});
