import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import { isValidAddress } from "./webClientTestUtils";

enum StorageMode {
  PRIVATE = "private",
  PUBLIC = "public",
}

interface NewAccountTestResult {
  id: string;
  nonce: string;
  vault_commitment: string;
  storage_commitment: string;
  code_commitment: string;
  is_faucet: boolean;
  is_regular_account: boolean;
  is_updatable: boolean;
  is_public: boolean;
  is_new: boolean;
}

// new_wallet tests
// =======================================================================================================

export const createNewWallet = async (
  storageMode: StorageMode,
  mutable: boolean,
  initSeed?: Uint8Array
): Promise<NewAccountTestResult> => {

  // Serialize initSeed for Puppeteer
  const serializedInitSeed = initSeed ? Array.from(initSeed) : null;

  return await testingPage.evaluate(
    async (_storageMode, _mutable, _serializedInitSeed) => {
      const client = window.client;
      const accountStorageMode =
        _storageMode === "private"
          ? window.AccountStorageMode.private()
          : window.AccountStorageMode.public();

      // Reconstruct Uint8Array inside the browser context
      const _initSeed = _serializedInitSeed ? new Uint8Array(_serializedInitSeed) : undefined;

      const newWallet = await client.new_wallet(accountStorageMode, _mutable, _initSeed);

      return {
        id: newWallet.id().to_string(),
        nonce: newWallet.nonce().to_string(),
        vault_commitment: newWallet.vault().commitment().to_hex(),
        storage_commitment: newWallet.storage().commitment().to_hex(),
        code_commitment: newWallet.code().commitment().to_hex(),
        is_faucet: newWallet.is_faucet(),
        is_regular_account: newWallet.is_regular_account(),
        is_updatable: newWallet.is_updatable(),
        is_public: newWallet.is_public(),
        is_new: newWallet.is_new(),
      };
    },
    storageMode,
    mutable,
    serializedInitSeed
  );
};

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
      const result = await createNewWallet(storageMode, mutable);

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
    const initSeed = new Uint8Array(32);
    crypto.getRandomValues(initSeed)

    const wallet1 = await createNewWallet(StorageMode.PUBLIC, false, initSeed);
    const wallet2 = await createNewWallet(StorageMode.PUBLIC, false, initSeed);

    console.log({wallet1})
    console.log({wallet2})

    expect(wallet1.id).to.equal(wallet2.id);
  })
});

// new_faucet tests
// =======================================================================================================

export const createNewFaucet = async (
  storageMode: StorageMode,
  nonFungible: boolean,
  tokenSymbol: string,
  decimals: number,
  maxSupply: bigint
): Promise<NewAccountTestResult> => {
  return await testingPage.evaluate(
    async (_storageMode, _nonFungible, _tokenSymbol, _decimals, _maxSupply) => {
      const client = window.client;
      const accountStorageMode =
        _storageMode === "private"
          ? window.AccountStorageMode.private()
          : window.AccountStorageMode.public();
      const newFaucet = await client.new_faucet(
        accountStorageMode,
        _nonFungible,
        _tokenSymbol,
        _decimals,
        _maxSupply
      );
      return {
        id: newFaucet.id().to_string(),
        nonce: newFaucet.nonce().to_string(),
        vault_commitment: newFaucet.vault().commitment().to_hex(),
        storage_commitment: newFaucet.storage().commitment().to_hex(),
        code_commitment: newFaucet.code().commitment().to_hex(),
        is_faucet: newFaucet.is_faucet(),
        is_regular_account: newFaucet.is_regular_account(),
        is_updatable: newFaucet.is_updatable(),
        is_public: newFaucet.is_public(),
        is_new: newFaucet.is_new(),
      };
    },
    storageMode,
    nonFungible,
    tokenSymbol,
    decimals,
    maxSupply
  );
};

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
