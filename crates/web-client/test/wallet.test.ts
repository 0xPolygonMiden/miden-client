import { createNewWallet, isValidAddress } from "./webClientTestUtils.js";

describe("wallet tests", () => {
  it("create a new wallet", async () => {
    const result = await createNewWallet("Private", false);

    isValidAddress(result);
  });
});
