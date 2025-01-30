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
  storageMode: StorageMode,
  mutable: boolean
) => {
  const serializedWalletSeed = Array.from(walletSeed);
  return await testingPage.evaluate(
    async (_serializedWalletSeed, _storageMode, _mutable) => {
      const client = window.client;
      const _walletSeed = new Uint8Array(_serializedWalletSeed);

      const accountStorageMode =
        _storageMode === "private"
          ? window.AccountStorageMode.private()
          : window.AccountStorageMode.public();

      await client.import_account_from_seed(
        _walletSeed,
        accountStorageMode,
        _mutable
      );
    },
    serializedWalletSeed,
    storageMode,
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

    await importWalletFromSeed(walletSeed, storageMode, mutable);

    const restoredBalance = await getAccountBalance(
      initialWallet.id,
      faucet.id
    );

    expect(restoredBalance.toString()).to.equal(initialBalance);
  });
});
