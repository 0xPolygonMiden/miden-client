import { createNewFaucet, isValidAddress } from "./webClientTestUtils.js";

describe("faucet tests", () => {
  it("create a new faucet", async () => {
    const result = await createNewFaucet(
      "Private",
      false,
      "DMX",
      "10",
      "1000000"
    );

    isValidAddress(result);
  });
});
