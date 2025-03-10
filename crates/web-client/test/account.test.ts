import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";

// GET_ACCOUNT TESTS
// =======================================================================================================

interface GetAccountSuccessResult {
  hashOfCreatedAccount: string;
  hashOfGetAccountResult: string;
  isAccountType: boolean | undefined;
}

export const getAccountOneMatch =
  async (): Promise<GetAccountSuccessResult> => {
    return await testingPage.evaluate(async () => {
      const client = window.client;
      const newAccount = await client.newWallet(
        window.AccountStorageMode.private(),
        true
      );
      const result = await client.getAccount(newAccount.id());

      return {
        hashOfCreatedAccount: newAccount.hash().toHex(),
        hashOfGetAccountResult: result.hash().toHex(),
        isAccountType: result instanceof window.Account,
      };
    });
  };

interface GetAccountFailureResult {
  hashOfGetAccountResult: string | undefined;
}

export const getAccountNoMatch = async (): Promise<GetAccountFailureResult> => {
  return await testingPage.evaluate(async () => {
    const client = window.client;
    const nonExistingAccountId = window.TestUtils.createMockAccountId();

    const result = await client.getAccount(nonExistingAccountId);

    return {
      hashOfGetAccountResult: result ? result.hash().toHex() : undefined,
    };
  });
};

describe("get_account tests", () => {
  it("retrieves an existing account", async () => {
    const result = await getAccountOneMatch();

    expect(result.hashOfCreatedAccount).to.equal(result.hashOfGetAccountResult);
    expect(result.isAccountType).to.be.true;
  });

  it("returns error attempting to retrieve a non-existing account", async () => {
    const result = await getAccountNoMatch();

    expect(result.hashOfGetAccountResult).to.be.undefined;
  });
});

// GET_ACCOUNTS TESTS
// =======================================================================================================

interface GetAccountsSuccessResult {
  hashesOfCreatedAccounts: string[];
  hashesOfGetAccountsResult: string[];
  resultTypes: boolean[];
}

export const getAccountsManyMatches =
  async (): Promise<GetAccountsSuccessResult> => {
    return await testingPage.evaluate(async () => {
      const client = window.client;
      const newAccount1 = await client.newWallet(
        window.AccountStorageMode.private(),
        true
      );
      const newAccount2 = await client.newWallet(
        window.AccountStorageMode.private(),
        true
      );
      const hashesOfCreatedAccounts = [
        newAccount1.hash().toHex(),
        newAccount2.hash().toHex(),
      ];

      const result = await client.getAccounts();

      const hashesOfGetAccountsResult = [];
      const resultTypes = [];

      for (let i = 0; i < result.length; i++) {
        hashesOfGetAccountsResult.push(result[i].hash().toHex());
        resultTypes.push(result[i] instanceof window.AccountHeader);
      }

      return {
        hashesOfCreatedAccounts: hashesOfCreatedAccounts,
        hashesOfGetAccountsResult: hashesOfGetAccountsResult,
        resultTypes: resultTypes,
      };
    });
  };

export const getAccountsNoMatches =
  async (): Promise<GetAccountsSuccessResult> => {
    return await testingPage.evaluate(async () => {
      const client = window.client;

      const result = await client.getAccounts();

      const hashesOfGetAccountsResult = [];
      const resultTypes = [];

      for (let i = 0; i < result.length; i++) {
        hashesOfGetAccountsResult.push(result[i].hash().toHex());
        resultTypes.push(result[i] instanceof window.AccountHeader);
      }

      return {
        hashesOfCreatedAccounts: [],
        hashesOfGetAccountsResult: hashesOfGetAccountsResult,
        resultTypes: resultTypes,
      };
    });
  };

describe("getAccounts tests", () => {
  it("retrieves all existing accounts", async () => {
    const result = await getAccountsManyMatches();

    for (let address of result.hashesOfGetAccountsResult) {
      expect(result.hashesOfCreatedAccounts.includes(address)).to.be.true;
    }
    expect(result.resultTypes).to.deep.equal([true, true]);
  });

  it("returns empty array when no accounts exist", async () => {
    const result = await getAccountsNoMatches();

    expect(result.hashesOfCreatedAccounts.length).to.equal(0);
    expect(result.hashesOfGetAccountsResult.length).to.equal(0);
    expect(result.resultTypes.length).to.equal(0);
  });
});
