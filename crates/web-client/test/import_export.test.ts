// TODO: Rename this / figure out rebasing with the other featuer which has import tests

import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import { clearStore, setupWalletAndFaucet } from "./webClientTestUtils";

const exportDb = async () => {
  return await testingPage.evaluate(async () => {
    const client = window.client;
    const db = await client.exportStore();
    const serialized = JSON.stringify(db);
    return serialized;
  });
};

const importDb = async (db: any) => {
  return await testingPage.evaluate(async (_db) => {
    const client = window.client;
    await client.forceImportStore(_db);
  }, db);
};

const getAccount = async (accountId: string) => {
  return await testingPage.evaluate(async (_accountId) => {
    const client = window.client;
    const accountId = window.AccountId.fromHex(_accountId);
    const account = await client.getAccount(accountId);
    return {
      accountId: account?.id().toString(),
      accountCommitment: account?.commitment().toHex(),
    };
  }, accountId);
};

describe("export and import the db", () => {
  it("export db with an account, find the account when re-importing", async () => {
    const { accountCommitment: initialAccountCommitment, accountId } =
      await setupWalletAndFaucet();
    const dbDump = await exportDb();

    await clearStore();

    await importDb(dbDump);

    const { accountCommitment } = await getAccount(accountId);

    expect(accountCommitment).to.equal(initialAccountCommitment);
  });
});
