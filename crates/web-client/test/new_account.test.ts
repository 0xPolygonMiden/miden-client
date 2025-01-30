import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import {
  createNewFaucet,
  createNewWallet,
  isValidAddress,
  NewAccountTestResult,
  StorageMode,
} from "./webClientTestUtils";

// new_wallet tests
// =======================================================================================================

describe("new_wallet tests", () => {
  const testCases = [
    {
      description: "creates a new private, immutable wallet",
      storageMode: StorageMode.PRIVATE,
      mutable: false,
      expected: {
        is_public: false,
        is_updatable: false,
      },
    },
    {
      description: "creates a new public, immutable wallet",
      storageMode: StorageMode.PUBLIC,
      mutable: false,
      expected: {
        is_public: true,
        is_updatable: false,
      },
    },
    {
      description: "creates a new private, mutable wallet",
      storageMode: StorageMode.PRIVATE,
      mutable: true,
      expected: {
        is_public: false,
        is_updatable: true,
      },
    },
    {
      description: "creates a new public, mutable wallet",
      storageMode: StorageMode.PUBLIC,
      mutable: true,
      expected: {
        is_public: true,
        is_updatable: true,
      },
    },
  ];

  testCases.forEach(({ description, storageMode, mutable, expected }) => {
    it(description, async () => {
      const result = await createNewWallet({ storageMode, mutable });

      isValidAddress(result.id);
      expect(result.nonce).to.equal("0");
      isValidAddress(result.vault_commitment);
      isValidAddress(result.storage_commitment);
      isValidAddress(result.code_commitment);
      expect(result.is_faucet).to.equal(false);
      expect(result.is_regular_account).to.equal(true);
      expect(result.is_updatable).to.equal(expected.is_updatable);
      expect(result.is_public).to.equal(expected.is_public);
      expect(result.is_new).to.equal(true);
    });
  });

  it("Constructs the same account when given the same init seed", async () => {
    const clientSeed = new Uint8Array(32);
    crypto.getRandomValues(clientSeed);

    // Isolate the client instance both times to ensure the outcome is deterministic
    await createNewWallet({
      storageMode: StorageMode.PUBLIC,
      mutable: false,
      clientSeed,
      isolatedClient: true,
    });

    // This should fail, as the wallet is already tracked within the same browser context
    await expect(
      createNewWallet({
        storageMode: StorageMode.PUBLIC,
        mutable: false,
        clientSeed,
        isolatedClient: true,
      })
    ).to.be.rejectedWith(/Failed to insert new wallet: AccountAlreadyTracked/);
  });
});

// new_faucet tests
// =======================================================================================================


describe("new_faucet tests", () => {
  const testCases = [
    {
      description: "creates a new private, fungible faucet",
      storageMode: StorageMode.PRIVATE,
      non_fungible: false,
      token_symbol: "DAG",
      decimals: 8,
      max_supply: BigInt(10000000),
      expected: {
        is_public: false,
        is_updatable: false,
        is_regular_account: false,
        is_faucet: true,
      },
    },
    {
      description: "creates a new public, fungible faucet",
      storageMode: StorageMode.PUBLIC,
      non_fungible: false,
      token_symbol: "DAG",
      decimals: 8,
      max_supply: BigInt(10000000),
      expected: {
        is_public: true,
        is_updatable: false,
        is_regular_account: false,
        is_faucet: true,
      },
    },
  ];

  testCases.forEach(
    ({
      description,
      storageMode,
      non_fungible,
      token_symbol,
      decimals,
      max_supply,
      expected,
    }) => {
      it(description, async () => {
        const result = await createNewFaucet(
          storageMode,
          non_fungible,
          token_symbol,
          decimals,
          max_supply
        );

        isValidAddress(result.id);
        expect(result.nonce).to.equal("0");
        isValidAddress(result.vault_commitment);
        isValidAddress(result.storage_commitment);
        isValidAddress(result.code_commitment);
        expect(result.is_faucet).to.equal(true);
        expect(result.is_regular_account).to.equal(false);
        expect(result.is_updatable).to.equal(false);
        expect(result.is_public).to.equal(expected.is_public);
        expect(result.is_new).to.equal(true);
      });
    }
  );

  it("throws an error when attempting to create a non-fungible faucet", async () => {
    await expect(
      createNewFaucet(StorageMode.PUBLIC, true, "DAG", 8, BigInt(10000000))
    ).to.be.rejectedWith("Non-fungible faucets are not supported yet");
  });

  it("throws an error when attempting to create a faucet with an invalid token symbol", async () => {
    await expect(
      createNewFaucet(
        StorageMode.PUBLIC,
        false,
        "INVALID_TOKEN",
        8,
        BigInt(10000000)
      )
    ).to.be.rejectedWith(
      `token symbol of length 13 is not between 1 and 6 characters long`
    );
  });
});
