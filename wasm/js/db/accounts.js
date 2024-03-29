import { 
    accountCodes, 
    accountStorages, 
    accountVaults, 
    accountAuths, 
    accounts 
} from './schema.js';

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
        let accountIdStr = accountId.toString();
        let authBlob = new Blob([auth]);

        // Prepare the data object to insert
        const data = {
            accountId: accountIdStr, // Using accountId as the key
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
        let accountIdStr = accountId.toString();
        let nonceStr = nonce.toString();
        let accountSeedBlob = new Blob([account_seed]);

        // Prepare the data object to insert
        const data = {
            id: accountIdStr, // Using accountId as the key
            codeRoot: code_root,
            storageRoot: storage_root,
            vaultRoot: vault_root,
            nonce: nonceStr,
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