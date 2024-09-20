import {
  createNewConsumeTransaction,
  createNewFaucet,
  createNewMintTransaction,
  createNewWallet,
  fetchCacheAccountAuth,
  getInputNotes,
  isValidAddress,
  syncState,
} from "./webClientTestUtils.js";

describe("notes tests", () => {
  it("get input notes", async () => {
    console.log("testGetInputNotes started");

    let targetAccountId = await createNewWallet("Private", true);
    let faucetId = await createNewFaucet(
      "Private",
      false,
      "DEN",
      "10",
      "1000000"
    );
    console.log("syncing state...");
    await syncState();
    // await new Promise((r) => setTimeout(r, 20000));

    console.log("fetching cache account auth...");
    await fetchCacheAccountAuth(faucetId);

    console.log("creating new mint transaction...");
    let mintTransactionResult = await createNewMintTransaction(
      targetAccountId,
      faucetId,
      "Private",
      "1000"
    );
    await new Promise((r) => setTimeout(r, 20000));
    await syncState();

    await fetchCacheAccountAuth(targetAccountId);

    let consumeTransactionResult = await createNewConsumeTransaction(
      targetAccountId,
      mintTransactionResult.created_note_ids
    );
    await new Promise((r) => setTimeout(r, 20000));
    await syncState();

    await getInputNotes();

    console.log("testGetInputNotes finished");
  });
});
