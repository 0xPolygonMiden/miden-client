async function setupIndexedDB() {
    const dbName = "MidenClientDB";
    const dbVersion = 1; // Use a higher version if you need to upgrade the DB structure in the future

    return new Promise((resolve, reject) => {
        // Open a connection to the database
        const request = indexedDB.open(dbName, dbVersion);

        // This event is only triggered when the database is created for the first time,
        // or when the version number increases
        request.onupgradeneeded = function(event) {
            const db = event.target.result;

            // Create an object store named "Greet" if it doesn't already exist -- FOR TESTING
            if (!db.objectStoreNames.contains("Greet")) {
                db.createObjectStore("Greet", { autoIncrement: true });
            }

            //// ACCOUNTS //// 

            // account_code object store
            if (!db.objectStoreNames.contains("account_code")) {
                db.createObjectStore("account_code", { keyPath: "root" });
            }

            // account_storage object store
            if (!db.objectStoreNames.contains("account_storage")) {
                db.createObjectStore("account_storage", { keyPath: "root" });
            }

            // account_vaults object store
            if (!db.objectStoreNames.contains("account_vaults")) {
                db.createObjectStore("account_vaults", { keyPath: "root" });
            }

            // Create account_auth object store with account_id as the keyPath
            if (!db.objectStoreNames.contains("account_auth")) {
                db.createObjectStore("account_auth", { keyPath: "account_id" });
            }

            // Create accounts object store with a single keyPath
            if (!db.objectStoreNames.contains("accounts")) {
                const accountsStore = db.createObjectStore("accounts", { keyPath: "id_nonce" });
                // Create indexes for foreign keys
                accountsStore.createIndex("code_root", "code_root", { unique: false });
                accountsStore.createIndex("storage_root", "storage_root", { unique: false });
                accountsStore.createIndex("vault_root", "vault_root", { unique: false });
            }
        };

        // Handle successful database open operation
        request.onsuccess = function(event) {
            const db = event.target.result;
            resolve(db);
        };

        // Handle errors when opening the database
        request.onerror = function(event) {
            console.error("Error opening database:", request.error);
            reject(request.error);
        };
    });
}

async function insertAccountCode(codeRoot, code, moduleArrayBuffer) {
    const dbName = "MidenClientDB";
    const storeName = "account_code";
    
    return new Promise((resolve, reject) => {
        // Open a connection to the database
        const request = indexedDB.open(dbName);

        request.onsuccess = function(event) {
            const db = event.target.result;
            const transaction = db.transaction(storeName, "readwrite");
            const store = transaction.objectStore(storeName);

            // Prepare the data object to insert
            const data = {
                root: codeRoot, // Using codeRoot as the key
                code: code,
                module: moduleArrayBuffer, // Assuming module is already a Uint8Array or similar
            };

            // Perform the insert
            const insertRequest = store.add(data);

            insertRequest.onsuccess = function() {
                resolve(`Successfully inserted code with root: ${codeRoot}`);
            };

            insertRequest.onerror = function() {
                reject(`Error inserting code with root: ${codeRoot}: ${insertRequest.error}`);
            };
        };

        request.onerror = function(event) {
            console.error("Error opening database:", request.error);
            reject(request.error);
        };
    });
}

async function insertAccountStorage(storageRoot, storageSlots) {
    const dbName = "MidenClientDB";
    const storeName = "account_storage";

    return new Promise((resolve, reject) => {
        // Open a connection to the database
        const request = indexedDB.open(dbName);

        request.onsuccess = function(event) {
            const db = event.target.result;
            const transaction = db.transaction(storeName, "readwrite");
            const store = transaction.objectStore(storeName);

            // Prepare the data object to insert
            const data = {
                root: storageRoot, // Using storageRoot as the key
                slots: storageSlots, // Assuming storageSlots is a serialized JSON string or similar format
            };

            // Perform the insert
            const insertRequest = store.add(data);

            insertRequest.onsuccess = function() {
                resolve(`Successfully inserted storage with root: ${storageRoot}`);
            };

            insertRequest.onerror = function() {
                reject(`Error inserting storage with root: ${storageRoot}: ${insertRequest.error}`);
            };
        };

        request.onerror = function(event) {
            console.error("Error opening database:", request.error);
            reject(request.error);
        };
    });
}

async function insertAccountAssetVault(vaultRoot, assets) {
    const dbName = "MidenClientDB";
    const storeName = "account_vaults";

    return new Promise((resolve, reject) => {
        // Open a connection to the database
        const request = indexedDB.open(dbName);

        request.onsuccess = function(event) {
            const db = event.target.result;
            const transaction = db.transaction(storeName, "readwrite");
            const store = transaction.objectStore(storeName);

            // Prepare the data object to insert
            const data = {
                root: vaultRoot, // Using vaultRoot as the key
                assets: assets, // Assuming assets is a serialized JSON string or similar format
            };

            // Perform the insert
            const insertRequest = store.add(data);

            insertRequest.onsuccess = function() {
                resolve(`Successfully inserted asset vault with root: ${vaultRoot}`);
            };

            insertRequest.onerror = function() {
                reject(`Error inserting asset vault with root: ${vaultRoot}: ${insertRequest.error}`);
            };
        };

        request.onerror = function(event) {
            console.error("Error opening database:", request.error);
            reject(request.error);
        };
    });
}

async function insertAccountAuth(accountId, authInfoSerialized) {
    const dbName = "MidenClientDB";
    const storeName = "account_auth";

    return new Promise((resolve, reject) => {
        // Open a connection to the database
        const request = indexedDB.open(dbName);

        request.onsuccess = function(event) {
            const db = event.target.result;
            const transaction = db.transaction(storeName, "readwrite");
            const store = transaction.objectStore(storeName);

            // Prepare the data object to insert
            const data = {
                account_id: accountId, // account_id as the key
                auth_info: authInfoSerialized, // Assuming authInfo is already serialized (e.g., JSON string)
            };

            // Perform the insert
            const insertRequest = store.add(data);

            insertRequest.onsuccess = function() {
                resolve(`Successfully inserted auth info for account ID: ${accountId}`);
            };

            insertRequest.onerror = function() {
                reject(`Error inserting auth info for account ID: ${accountId}: ${insertRequest.error}`);
            };
        };

        request.onerror = function(event) {
            console.error("Error opening database:", request.error);
            reject(request.error);
        };
    });
}

async function insertAccountRecord(id, codeRoot, storageRoot, vaultRoot, nonce, committed, accountSeedArrayBuffer) {
    const dbName = "MidenClientDB";
    const storeName = "accounts";

    // Convert `id` and `nonce` to strings to ensure precision is maintained without loss.
    const idStr = id.toString();
    const nonceStr = nonce.toString();

    return new Promise((resolve, reject) => {
        const request = indexedDB.open(dbName);

        request.onsuccess = function(event) {
            const db = event.target.result;
            const transaction = db.transaction(storeName, "readwrite");
            const store = transaction.objectStore(storeName);

            // Prepare the data object to insert, using the converted `id` and `nonce` for the `id_nonce` key
            const data = {
                id_nonce: `${idStr}_${nonceStr}`, // Concatenating `id` and `nonce` for the primary key
                code_root: codeRoot,
                storage_root: storageRoot,
                vault_root: vaultRoot,
                committed: committed,
                account_seed: accountSeedArrayBuffer, // Directly using `accountSeedArrayBuffer` as it's automatically converted to ArrayBuffer
            };

            const insertRequest = store.add(data);

            insertRequest.onsuccess = function() {
                resolve(`Successfully inserted account record for ID: ${idStr} with Nonce: ${nonceStr}`);
            };

            insertRequest.onerror = function() {
                reject(`Error inserting account record for ID: ${idStr} with Nonce: ${nonceStr}: ${insertRequest.error}`);
            };
        };

        request.onerror = function(event) {
            console.error("Error opening database:", request.error);
            reject(request.error);
        };
    });
}

async function insertGreeting(greeting) {
    return new Promise((resolve, reject) => {
        const openRequest = indexedDB.open("MidenClientDB", 1);

        openRequest.onsuccess = function(event) {
            const db = event.target.result;
            const transaction = db.transaction("Greet", "readwrite");
            const store = transaction.objectStore("Greet");
            const putRequest = store.put(greeting);

            putRequest.onsuccess = function() {
                resolve();
            };

            putRequest.onerror = function(e) {
                console.error("Error inserting greeting:", e.target.error);
                reject(new Error("Error inserting greeting"));
            };
        };

        openRequest.onerror = function(event) {
            reject(new Error("Error opening database"));
        };
    });
}

export { 
    setupIndexedDB, 
    insertGreeting, 
    insertAccountCode,
    insertAccountStorage,
    insertAccountAssetVault,
    insertAccountAuth,
    insertAccountRecord
 }