import { createNewFaucet, isValidAddress } from "./webClientTestUtils.js";

describe("faucet tests", () => {
  it("create a new faucet", async () => {
    const result = await createNewFaucet(
      "OffChain",
      false,
      "DMX",
      "10",
      "1000000"
    );

    isValidAddress(result);
  });
});
