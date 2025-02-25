import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import {
  clearStore,
  createNewFaucet,
  createNewWallet,
  fundAccountFromFaucet,
  getAccountBalance,
  StorageMode,
} from "./webClientTestUtils";

const importWalletFromSeed = async (
  walletSeed: Uint8Array,
  mutable: boolean
) => {
  const serializedWalletSeed = Array.from(walletSeed);
  return await testingPage.evaluate(
    async (_serializedWalletSeed, _mutable) => {
      const client = window.client;
      const _walletSeed = new Uint8Array(_serializedWalletSeed);

      await client.import_public_account_from_seed(_walletSeed, _mutable);
    },
    serializedWalletSeed,
    mutable
  );
};

describe("import from seed", () => {
  it("should import same public account from seed", async () => {
    const walletSeed = new Uint8Array(32);
    crypto.getRandomValues(walletSeed);

    const mutable = false;
    const storageMode = StorageMode.PUBLIC;

    const initialWallet = await createNewWallet({
      storageMode,
      mutable,
      walletSeed,
    });
    const faucet = await createNewFaucet();

    const result = await fundAccountFromFaucet(initialWallet.id, faucet.id);
    const initialBalance = result.targetAccountBalanace;

    // Deleting the account
    await clearStore();

    await importWalletFromSeed(walletSeed, mutable);

    const restoredBalance = await getAccountBalance(
      initialWallet.id,
      faucet.id
    );

    expect(restoredBalance.toString()).to.equal(initialBalance);
  });
});
