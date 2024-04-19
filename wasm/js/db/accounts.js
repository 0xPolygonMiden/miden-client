import { 
    accountCodes, 
    accountStorages, 
    accountVaults, 
    accountAuths, 
    accounts 
} from './schema.js';

// GET FUNCTIONS
export async function getAccountStub(
    accountId
) {
    try {
        // Fetch all records matching the given id
        const allMatchingRecords = await accounts
          .where('id')
          .equals(accountId)
          .toArray();
    
        if (allMatchingRecords.length === 0) {
          console.log('No records found for given ID.');
          return null; // No records found
        }
    
        // Convert nonce to BigInt and sort
        // Note: This assumes all nonces are valid BigInt strings.
        const sortedRecords = allMatchingRecords.sort((a, b) => {
          const bigIntA = BigInt(a.nonce);
          const bigIntB = BigInt(b.nonce);
          return bigIntA > bigIntB ? -1 : bigIntA < bigIntB ? 1 : 0;
        });
    
        // The first record is the most recent one due to the sorting
        const mostRecentRecord = sortedRecords[0];
        console.log('Most recent record found:', mostRecentRecord);

        let accountSeedBase64 = null;
        if (mostRecentRecord.accountSeed) {
            // Ensure accountSeed is processed as a Uint8Array and converted to Base64
            let accountSeedArrayBuffer = await mostRecentRecord.accountSeed.arrayBuffer();
            let accountSeedArray = new Uint8Array(accountSeedArrayBuffer);
            accountSeedBase64 = uint8ArrayToBase64(accountSeedArray);
        }
        const accountStub = {
            id: mostRecentRecord.id,
            nonce: mostRecentRecord.nonce,
            vault_root: mostRecentRecord.vaultRoot,
            storage_root: mostRecentRecord.storageRoot,
            code_root: mostRecentRecord.codeRoot,
            account_seed: accountSeedBase64
        }
        return accountStub;
      } catch (error) {
        console.error('Error fetching most recent account record:', error);
        throw error; // Re-throw the error for further handling
      }
}

export async function getAllAccountStubs() {
    try {
        // Fetch all records
        const allRecords = await accounts.toArray();
        
        // Use a Map to track the latest record for each id based on nonce
        const latestRecordsMap = new Map();

        allRecords.forEach(record => {
            const existingRecord = latestRecordsMap.get(record.id);
            if (!existingRecord || BigInt(record.nonce) > BigInt(existingRecord.nonce)) {
                latestRecordsMap.set(record.id, record);
            }
        });

        // Extract the latest records from the Map
        const latestRecords = Array.from(latestRecordsMap.values());

        console.log('Latest account stub for each id:', latestRecords);
        const resultObject = await Promise.all(latestRecords.map(async record => {
            let accountSeedBase64 = null;
            if (record.accountSeed) {
                // Ensure accountSeed is processed as a Uint8Array and converted to Base64
                let accountSeedArrayBuffer = await record.accountSeed.arrayBuffer();
                let accountSeedArray = new Uint8Array(accountSeedArrayBuffer);
                accountSeedBase64 = uint8ArrayToBase64(accountSeedArray);
            }

            return {
                id: record.id,
                nonce: record.nonce,
                vault_root: record.vaultRoot,
                storage_root: record.storageRoot,
                code_root: record.codeRoot,
                account_seed: accountSeedBase64 // Now correctly formatted as Base64
            };
        }));

        console.log(resultObject[0]);
        return resultObject;
    } catch (error) {
        console.error('Error fetching all latest account stubs:', error);
        throw error;
    }
}

export async function getAccountCode(
    codeRoot
) {
    try {
        // Fetch all records matching the given root
        const allMatchingRecords = await accountCodes
            .where('root')
            .equals(codeRoot)
            .toArray();

        if (allMatchingRecords.length === 0) {
            console.log('No records found for given code root.');
            return null; // No records found
        }

        // The first record is the only one due to the uniqueness constraint
        const codeRecord = allMatchingRecords[0];
        console.log('Code record found:', codeRecord);

        // Convert the module Blob to an ArrayBuffer
        const moduleArrayBuffer = await codeRecord.module.arrayBuffer();
        const moduleArray = new Uint8Array(moduleArrayBuffer);
        const moduleBase64 = uint8ArrayToBase64(moduleArray);
        return {
            root: codeRecord.root,
            procedures: codeRecord.procedures,
            module: moduleBase64,
        };
    } catch (error) {
        console.error('Error fetching code record:', error);
        throw error; // Re-throw the error for further handling
    }
}

export async function getAccountAssetVault(
    vaultRoot
) {
    try {
        // Fetch all records matching the given root
        const allMatchingRecords = await accountVaults
            .where('root')
            .equals(vaultRoot)
            .toArray();

        if (allMatchingRecords.length === 0) {
            console.log('No records found for given code root.');
            return null; // No records found
        }

        // The first record is the only one due to the uniqueness constraint
        const vaultRecord = allMatchingRecords[0];
        console.log('Vault record found:', vaultRecord);

        return {
            root: vaultRecord.root,
            assets: vaultRecord.assets
        };
    } catch (error) {
        console.error('Error fetching code record:', error);
        throw error; // Re-throw the error for further handling
    }
}

export async function getAccountStorage(
    storageRoot
) {
    try {
        // Fetch all records matching the given root
        const allMatchingRecords = await accountStorages
            .where('root')
            .equals(storageRoot)
            .toArray();

        if (allMatchingRecords.length === 0) {
            console.log('No records found for given code root.');
            return null; // No records found
        }

        // The first record is the only one due to the uniqueness constraint
        const storageRecord = allMatchingRecords[0];
        console.log('Storage record found:', storageRecord);

        // Convert the module Blob to an ArrayBuffer
        const storageArrayBuffer = await storageRecord.slots.arrayBuffer();
        const storageArray = new Uint8Array(storageArrayBuffer);
        const storageBase64 = uint8ArrayToBase64(storageArray);
        return {
            root: storageRecord.root,
            storage: storageBase64
        };
    } catch (error) {
        console.error('Error fetching code record:', error);
        throw error; // Re-throw the error for further handling
    }
}

export async function getAccountAuth(
    accountId
) {
    try {
        // Fetch all records matching the given id
        const allMatchingRecords = await accountAuths
            .where('accountId')
            .equals(accountId)
            .toArray();

        if (allMatchingRecords.length === 0) {
            console.log('No records found for given account ID.');
            return null; // No records found
        }

        // The first record is the only one due to the uniqueness constraint
        const authRecord = allMatchingRecords[0];
        console.log('Auth record found:', authRecord);

        // Convert the authInfo Blob to an ArrayBuffer
        const authInfoArrayBuffer = await authRecord.authInfo.arrayBuffer();
        const authInfoArray = new Uint8Array(authInfoArrayBuffer);
        const authInfoBase64 = uint8ArrayToBase64(authInfoArray);
        return {
            id: authRecord.accountId,
            auth_info: authInfoBase64
        };
    } catch (err) {
        console.error('Error fetching account auth:', error);
        throw error; // Re-throw the error for further handling
    }
}

export async function getAccountIds() {
    try {
        let allIds = new Set(); // Use a Set to ensure uniqueness

        // Iterate over each account entry
        await accounts.each(account => {
            allIds.add(account.id); // Assuming 'account' has an 'id' property
        });

        return Array.from(allIds); // Convert back to array to return a list of unique IDs
    } catch (error) {
        console.error("Failed to retrieve account IDs: ", error);
        throw error; // Or handle the error as fits your application's error handling strategy
    }
}

// INSERT FUNCTIONS

export async function insertAccountCode(
    codeRoot, 
    code, 
    module
) {
    try {
        // Create a Blob from the ArrayBuffer
        const moduleBlob = new Blob([module]);

        // Prepare the data object to insert
        const data = {
            root: codeRoot, // Using codeRoot as the key
            procedures: code,
            module: moduleBlob, // Blob created from ArrayBuffer
        };

        // Perform the insert using Dexie
        await accountCodes.add(data);
    } catch (error) {
        console.error(`Error inserting code with root: ${codeRoot}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

export async function insertAccountStorage(
    storageRoot, 
    storageSlots
) {
    try {
        const storageSlotsBlob = new Blob([storageSlots]);

        // Prepare the data object to insert
        const data = {
            root: storageRoot, // Using storageRoot as the key
            slots: storageSlotsBlob, // Blob created from ArrayBuffer
        };

        // Perform the insert using Dexie
        await accountStorages.add(data);
    } catch (error) {
        console.error(`Error inserting storage with root: ${storageRoot}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

export async function insertAccountAssetVault(
    vaultRoot, 
    assets
) {
    try {
        // Prepare the data object to insert
        const data = {
            root: vaultRoot, // Using vaultRoot as the key
            assets: assets,
        };

        // Perform the insert using Dexie
        await accountVaults.add(data);
    } catch (error) {
        console.error(`Error inserting vault with root: ${vaultRoot}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

export async function insertAccountAuth(
    accountId, 
    authInfo
) {
    try {
        let authInfoBlob = new Blob([new Uint8Array(authInfo)]);

        // Prepare the data object to insert
        const data = {
            accountId: accountId, // Using accountId as the key
            authInfo: authInfoBlob,
        };

        // Perform the insert using Dexie
        await accountAuths.add(data);
    } catch (error) {
        console.error(`Error inserting auth for account: ${accountId}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

export async function insertAccountRecord(
    accountId,
    code_root,
    storage_root,
    vault_root,
    nonce,
    committed,
    account_seed
) {
    try {
        let accountSeedBlob = null;
        console.log(account_seed);
        if (account_seed) {
            console.log(account_seed)
            accountSeedBlob = new Blob([new Uint8Array(account_seed)]);
        }
        

        // Prepare the data object to insert
        const data = {
            id: accountId, // Using accountId as the key
            codeRoot: code_root,
            storageRoot: storage_root,
            vaultRoot: vault_root,
            nonce: nonce,
            committed: committed,
            accountSeed: accountSeedBlob,
        };

        // Perform the insert using Dexie
        await accounts.add(data);
    } catch (error) {
        console.error(`Error inserting account: ${accountId}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

function uint8ArrayToBase64(bytes) {
    const binary = bytes.reduce((acc, byte) => acc + String.fromCharCode(byte), '');
    return btoa(binary);
}