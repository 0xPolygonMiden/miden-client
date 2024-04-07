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

        const account_seed_array_buffer = await data.account_seed.arrayBuffer();
        const accountStub = {
            id: data.id,
            nonce: data.nonce,
            vault_root: data.vaultRoot,
            storage_root: data.storageRoot,
            code_root: data.codeRoot,
            account_seed: new Uint8Array(account_seed_array_buffer)
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
        return latestRecords.map(record => {
            // Convert fields as necessary, assuming account_seed is already in the correct format
            return {
                id: record.id,
                nonce: record.nonce,
                vault_root: record.vaultRoot,
                storage_root: record.storageRoot,
                code_root: record.codeRoot,
                account_seed: record.account_seed // Adjust based on your actual data structure
            };
        });
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
        return {
            root: codeRecord.root,
            procedures: codeRecord.procedures,
            module: new Uint8Array(moduleArrayBuffer),
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
        console.log('Vault record found:', vaultRecord);

        // Convert the module Blob to an ArrayBuffer
        const storageArrayBuffer = await storageRecord.storage.arrayBuffer();
        return {
            root: storageRecord.root,
            storage: storageArrayBuffer
        };
    } catch (error) {
        console.error('Error fetching code record:', error);
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
        return `Successfully inserted code with root: ${codeRoot}`;
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
        return `Successfully inserted storage with root: ${storageRoot}`;
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
        return `Successfully inserted vault with root: ${vaultRoot}`;
    } catch (error) {
        console.error(`Error inserting vault with root: ${vaultRoot}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

export async function insertAccountAuth(
    accountId, 
    auth
) {
    try {
        let authBlob = new Blob([auth]);

        // Prepare the data object to insert
        const data = {
            accountId: accountId, // Using accountId as the key
            auth: authBlob,
        };

        // Perform the insert using Dexie
        await accountAuths.add(data);
        return `Successfully inserted auth for account: ${accountId}`;
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
        let accountSeedBlob = new Blob([account_seed]);

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
        return `Successfully inserted account: ${accountId}`;
    } catch (error) {
        console.error(`Error inserting account: ${accountId}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}