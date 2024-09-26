import { expect } from 'chai';
import { testingPage } from "./mocha.global.setup.mjs";

// get_account tests
// =======================================================================================================

interface GetAccountSuccessResult {
    addressOfCreatedAccount: string;
    addressOfGetAccountResult: string;
    isAccountType: boolean | undefined;
}

export const getAccountOneMatch = async (): Promise<GetAccountSuccessResult> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }

        const client = window.client;
        const newAccount = await client.new_wallet(window.AccountStorageMode.private(), true);
        const result = await client.get_account(newAccount.id().to_string());

        return {
            addressOfCreatedAccount: newAccount.id().to_string(),
            addressOfGetAccountResult: result.id().to_string(),
            isAccountType: result instanceof window.Account
        }
    });
};

export const getAccountNoMatch = async (): Promise<void> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }

        const client = window.client;
        await client.get_account("0x1111111111111111");
    });
};

describe("get_account tests", () => {
    it("retrieves an existing account", async () => {
        const result = await getAccountOneMatch();

        expect(result.addressOfCreatedAccount).to.equal(result.addressOfGetAccountResult);
        expect(result.isAccountType).to.be.true;
    });

    it("returns error attempting to retrieve a non-existing account", async () => {
        await expect(
            getAccountNoMatch()
          ).to.be.rejectedWith("Failed to get account: Store error: Account data was not found for Account Id 0x1111111111111111");
    });
  });

// get_account tests
// =======================================================================================================

interface GetAccountsSuccessResult {
    addressesOfCreatedAccounts: string[];
    addressesOfGetAccountsResult: string[];
    resultTypes: boolean[];
}

export const getAccountsManyMatches = async (): Promise<GetAccountsSuccessResult> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }

        const client = window.client;
        const newAccount1 = await client.new_wallet(window.AccountStorageMode.private(), true);
        const newAccount2 = await client.new_wallet(window.AccountStorageMode.private(), true);
        const addressesOfCreatedAccounts = [newAccount1.id().to_string(), newAccount2.id().to_string()];
        
        const result = await client.get_accounts();
        
        const addressesOfGetAccountsResult = [];
        const resultTypes = [];

        for (let i = 0; i < result.length; i++) {
            addressesOfGetAccountsResult.push(result[i].id().to_string());
            resultTypes.push(result[i] instanceof window.AccountHeader);
        }

        return {
            addressesOfCreatedAccounts: addressesOfCreatedAccounts,
            addressesOfGetAccountsResult: addressesOfGetAccountsResult,
            resultTypes: resultTypes
        }
    });
};

export const getAccountsNoMatches = async (): Promise<GetAccountsSuccessResult> => {
    return await testingPage.evaluate(async () => {
        await window.create_client();

        const client = window.client;
            
        const result = await client.get_accounts();
        
        const addressesOfGetAccountsResult = [];
        const resultTypes = [];

        for (let i = 0; i < result.length; i++) {
            addressesOfGetAccountsResult.push(result[i].id().to_string());
            resultTypes.push(result[i] instanceof window.AccountHeader);
        }

        return {
            addressesOfCreatedAccounts: [],
            addressesOfGetAccountsResult: addressesOfGetAccountsResult,
            resultTypes: resultTypes
        }
    });
};

describe("get_accounts tests", () => {
    beforeEach(async () => {
        await testingPage.evaluate(async () => {
            // Open a connection to the list of databases
            const databases = await indexedDB.databases();
            for (const db of databases) {
                // Delete each database by name
                indexedDB.deleteDatabase(db.name!);
            }
        });
    });

    it("retrieves all existing accounts", async () => {
        const result = await getAccountsManyMatches();

        for (let address of result.addressesOfGetAccountsResult) {
            expect(result.addressesOfCreatedAccounts.includes(address)).to.be.true;
        }
        expect(result.resultTypes).to.deep.equal([true, true]);
    });

    it("returns empty array when no accounts exist", async () => {
        const result = await getAccountsNoMatches();

        expect(result.addressesOfCreatedAccounts.length).to.equal(0);
        expect(result.addressesOfGetAccountsResult.length).to.equal(0);
        expect(result.resultTypes.length).to.equal(0);
    });
});

// get_account_auth tests
// =======================================================================================================

interface GetAccountAuthSuccessResult {
    publicKey: any;
    secretKey: any;
    isAuthSecretKeyType: boolean | undefined;
}

export const getAccountAuth = async (): Promise<GetAccountAuthSuccessResult> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }

        const client = window.client;
        const newAccount = await client.new_wallet(window.AccountStorageMode.private(), true);
        
        const result = await client.get_account_auth(newAccount.id().to_string());

        return {
            publicKey: result.get_rpo_falcon_512_public_key_as_word(),
            secretKey: result.get_rpo_falcon_512_secret_key_as_felts(),
            isAuthSecretKeyType: result instanceof window.AuthSecretKey
        }
    });
};

export const getAccountAuthNoMatch = async (): Promise<void> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }

        const client = window.client;
        
        await client.get_account_auth("0x1111111111111111");
    });
};

describe("get_account_auth tests", () => {
    it("retrieves an existing account auth", async () => {
        const result = await getAccountAuth();

        expect(result.publicKey).to.not.be.empty;
        expect(result.secretKey).to.not.be.empty;
        expect(result.isAuthSecretKeyType).to.be.true;
    });

    it("returns error attempting to retrieve a non-existing account auth", async () => {
        await expect(
            getAccountAuthNoMatch()
          ).to.be.rejectedWith("Failed to get account auth: Store error: Account data was not found for Account Id 0x1111111111111111");
    });
});

// fetch_and_cache_account_auth_by_pub_key tests
// =======================================================================================================

interface FetchAndCacheAccountAuthByPubKeySuccessResult {
    publicKey: any;
    secretKey: any;
    isAuthSecretKeyType: boolean | undefined;
}

export const fetchAndCacheAccountAuthByPubKey = async (): Promise<FetchAndCacheAccountAuthByPubKeySuccessResult> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }

        const client = window.client;
        const newAccount = await client.new_wallet(window.AccountStorageMode.private(), true);
        
        const result = await client.fetch_and_cache_account_auth_by_pub_key(newAccount.id().to_string());

        return {
            publicKey: result.get_rpo_falcon_512_public_key_as_word(),
            secretKey: result.get_rpo_falcon_512_secret_key_as_felts(),
            isAuthSecretKeyType: result instanceof window.AuthSecretKey
        }
    });
};

export const fetchAndCacheAccountAuthByPubKeyNoMatch = async (): Promise<FetchAndCacheAccountAuthByPubKeySuccessResult> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }

        const client = window.client;
        
        const result = await client.fetch_and_cache_account_auth_by_pub_key("0x1111111111111111");

        return {
            publicKey: result.get_rpo_falcon_512_public_key_as_word(),
            secretKey: result.get_rpo_falcon_512_secret_key_as_felts(),
            isAuthSecretKeyType: result instanceof window.AuthSecretKey
        }
    });
};

describe("fetch_and_cache_account_auth_by_pub_key tests", () => {
    it("retrieves an existing account auth and caches it", async () => {
        const result = await fetchAndCacheAccountAuthByPubKey();

        expect(result.publicKey).to.not.be.empty;
        expect(result.secretKey).to.not.be.empty;
        expect(result.isAuthSecretKeyType).to.be.true;
    });

    it("returns error attempting to retrieve/cache a non-existing account auth", async () => {
        await expect(
            fetchAndCacheAccountAuthByPubKeyNoMatch()
          ).to.be.rejectedWith("Failed to fetch and cache account auth: Account data was not found for Account Id 0x1111111111111111");
    });
});
