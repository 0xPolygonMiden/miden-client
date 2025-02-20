// TODO: Rename this / figure out rebasing with the other featuer which has import tests

import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import { clearStore, setupWalletAndFaucet } from "./webClientTestUtils";

const exportDb = async () => {
  return await testingPage.evaluate(async () => {
    const client = window.client;
    const db = await client.export_store();
    const serialized = JSON.stringify(db);
    console.log("lenth: ", serialized.length);
    return serialized;
  });
};

const importDb = async (db: any) => {
  return await testingPage.evaluate(async (_db) => {
    const client = window.client;
    await client.import_store(_db);
  }, db);
};

const getAccount = async (accountId: string) => {
  return await testingPage.evaluate(async (_accountId) => {
    const client = window.client;
    const accountId = window.AccountId.from_hex(_accountId);
    const account = await client.get_account(accountId);
    return {
      accountId: account?.id().to_string(),
    };
  }, accountId);
};

describe("export and import the db", () => {
  it("export db with an account, find the account when re-importing", async () => {
    const { accountId: initialAccountId, faucetId } =
      await setupWalletAndFaucet();
    const dbDump = await exportDb();

    await clearStore();

    await importDb(dbDump);

    const { accountId } = await getAccount(initialAccountId);

    expect(accountId).to.equal(initialAccountId);
  });
});
