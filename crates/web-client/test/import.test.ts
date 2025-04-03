import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import {
  clearStore,
  createNewFaucet,
  createNewWallet,
  fundAccountFromFaucet,
  getAccount,
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
      await client.syncState();
      const _walletSeed = new Uint8Array(_serializedWalletSeed);
      const account = await client.importPublicAccountFromSeed(
        _walletSeed,
        _mutable
      );
      return {
        accountId: account.id().toString(),
        accountCommitment: account.commitment().toHex(),
      };
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

    const { commitment: initialCommitment } = await getAccount(
      initialWallet.id
    );

    // Deleting the account
    await clearStore();

    const { accountId: restoredAccountId } = await importWalletFromSeed(
      walletSeed,
      mutable
    );

    expect(restoredAccountId).to.equal(initialWallet.id);

    const { commitment: restoredAccountCommitment } = await getAccount(
      initialWallet.id
    );

    const restoredBalance = await getAccountBalance(
      initialWallet.id,
      faucet.id
    );

    expect(restoredBalance.toString()).to.equal(initialBalance);
    expect(restoredAccountCommitment).to.equal(initialCommitment);
  });
});
