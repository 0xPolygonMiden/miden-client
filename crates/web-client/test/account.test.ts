import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";

// GET_ACCOUNT TESTS
// =======================================================================================================

interface GetAccountSuccessResult {
  commitmentOfCreatedAccount: string;
  commitmentOfGetAccountResult: string;
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
        commitmentOfCreatedAccount: newAccount.commitment().toHex(),
        commitmentOfGetAccountResult: result.commitment().toHex(),
        isAccountType: result instanceof window.Account,
      };
    });
  };

interface GetAccountFailureResult {
  commitmentOfGetAccountResult: string | undefined;
}

export const getAccountNoMatch = async (): Promise<GetAccountFailureResult> => {
  return await testingPage.evaluate(async () => {
    const client = window.client;
    const nonExistingAccountId = window.TestUtils.createMockAccountId();

    const result = await client.getAccount(nonExistingAccountId);

    return {
      commitmentOfGetAccountResult: result
        ? result.commitment().toHex()
        : undefined,
    };
  });
};

describe("get_account tests", () => {
  it("retrieves an existing account", async () => {
    const result = await getAccountOneMatch();

    expect(result.commitmentOfCreatedAccount).to.equal(
      result.commitmentOfGetAccountResult
    );
    expect(result.isAccountType).to.be.true;
  });

  it("returns error attempting to retrieve a non-existing account", async () => {
    const result = await getAccountNoMatch();

    expect(result.commitmentOfGetAccountResult).to.be.undefined;
  });
});

// GET_ACCOUNTS TESTS
// =======================================================================================================

interface GetAccountsSuccessResult {
  commitmentsOfCreatedAccounts: string[];
  commitmentsOfGetAccountsResult: string[];
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
      const commitmentsOfCreatedAccounts = [
        newAccount1.commitment().toHex(),
        newAccount2.commitment().toHex(),
      ];

      const result = await client.getAccounts();

      const commitmentsOfGetAccountsResult = [];
      const resultTypes = [];

      for (let i = 0; i < result.length; i++) {
        commitmentsOfGetAccountsResult.push(result[i].commitment().toHex());
        resultTypes.push(result[i] instanceof window.AccountHeader);
      }

      return {
        commitmentsOfCreatedAccounts: commitmentsOfCreatedAccounts,
        commitmentsOfGetAccountsResult: commitmentsOfGetAccountsResult,
        resultTypes: resultTypes,
      };
    });
  };

export const getAccountsNoMatches =
  async (): Promise<GetAccountsSuccessResult> => {
    return await testingPage.evaluate(async () => {
      const client = window.client;

      const result = await client.getAccounts();

      const commitmentsOfGetAccountsResult = [];
      const resultTypes = [];

      for (let i = 0; i < result.length; i++) {
        commitmentsOfGetAccountsResult.push(result[i].commitment().toHex());
        resultTypes.push(result[i] instanceof window.AccountHeader);
      }

      return {
        commitmentsOfCreatedAccounts: [],
        commitmentsOfGetAccountsResult: commitmentsOfGetAccountsResult,
        resultTypes: resultTypes,
      };
    });
  };

describe("getAccounts tests", () => {
  it("retrieves all existing accounts", async () => {
    const result = await getAccountsManyMatches();

    for (let address of result.commitmentsOfGetAccountsResult) {
      expect(result.commitmentsOfCreatedAccounts.includes(address)).to.be.true;
    }
    expect(result.resultTypes).to.deep.equal([true, true]);
  });

  it("returns empty array when no accounts exist", async () => {
    const result = await getAccountsNoMatches();

    expect(result.commitmentsOfCreatedAccounts.length).to.equal(0);
    expect(result.commitmentsOfGetAccountsResult.length).to.equal(0);
    expect(result.resultTypes.length).to.equal(0);
  });
});
