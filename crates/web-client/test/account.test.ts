import { expect } from "chai";
import {
  createNewWallet,
  getAccount,
  getAccounts,
} from "./webClientTestUtils.js";

describe("account tests", () => {
  it("get accounts", async () => {
    const accountId = await createNewWallet("OffChain", false);
    const accounts = await getAccounts();
    expect(accounts.find((acc) => acc.id === accountId)).to.be.not.null;
  });

  it("get account", async () => {
    const accountId = await createNewWallet("OffChain", false);
    const result = await getAccount(accountId);
    expect(result).to.be.equal(accountId);
  });
});
