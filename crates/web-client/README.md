# @demox-labs/miden-sdk
The @demox-labs/miden-sdk is a toolkit designed for interacting with the Miden virtual machine. It offers essential tools and functionalities for developers aiming to integrate or utilize Miden VM capabilities in their applications.

## Installation
To install the package via npm, run the following command:

```javascript
npm i @demox-labs/miden-sdk
```

For yarn:
```javascript
yarn add @demox-labs/miden-sdk
```

## Usage

```typescript
import { WebClient } from "@demox-labs/miden-sdk";

const webClient = new WebClient();
await webClient.create_client();

// Use WebClient to create accounts, notes, transactions, etc.
// This will create a mutable, off-chain account and store it in IndexedDB
const accountId = await webClient.new_wallet("OffChain", true);
```

## Examples
### The WebClient
The WebClient is your gateway to creating and interacting with anything miden vm related.
Example:
```typescript
// Creates a new WebClient instance which can then be configured after
const webClient = new WebClient();

// Creates the internal client of a previously instantiated WebClient.
// Can provide `node_url` as an optional parameter. Defaults to "http://18.203.155.106:57291" which is the URL
// of the remote miden node.
await webClient.create_client();
```
Example specifying a specific node URL:
```typescript
const webClient = new WebClient();

let remote_node_url = "http://18.203.155.106:57291"
await webClient.create_client(remote_node_url);
```

### Accounts
You can use the WebClient to create and retrieve account information.
```typescript
const webClient = new WebClient();
await webClient.create_client();

/**
 * Creates a new wallet account.
 * 
 * @param storage_mode String. Either "OffChain" or "OnChain".
 * @param mutable Boolean. Whether the wallet code is mutable or not
 * 
 * Returns: Wallet Id
 */
const walletId = await webClient.new_wallet("OffChain", true);

/**
 * Creates a new faucet account.
 * 
 * @param storage_mode String. Either "OffChain" or "OnChain".
 * @param non_fungible Boolean. Whether the faucet is non_fungible or not. NOTE: Non-fungible faucets are not supported yet
 * @param token_symbol String. Token symbol of the token the faucet creates
 * @param decimals String. Decimal precision of token.
 * @param max_supply String. Maximum token supply
 */ 
const faucetId = await webClient.new_faucet("OffChain", true, "TOK", 6, 1_000_000)

/**
 * Returns all accounts. Both wallets and faucets. Returns the following object per account
 * {
 *   id: string
 *   nonce: string
 *   vault_root: string
 *   storage_root: string
 *   code_root: string
 * }/
const accounts = await webClient.get_accounts()
console.log(accounts[0].id) // Prints account id of first account retrieved as hex value

// Gets a single account by id
const account = await webClient.get_account("0x9258fec00ad6d9bc");

// Imports an account. This example adds a simple button to an HTML page, creates a listener for an account file selection, serializes that file into bytes, then calls the client to import it. 
<label for="accountFileInput" class="custom-file-upload">
    Choose Account File
</label>
<input type="file" id="accountFileInput" style="display: none;">
document.getElementById('accountFileInput').addEventListener('change', function(event) {
    const file = event.target.files[0];
    if (file) {
        const reader = new FileReader();

        reader.onload = async function(e) {
            let webClient = await createMidenWebClient();
            const arrayBuffer = e.target.result;
            const byteArray = new Uint8Array(arrayBuffer);

            await webClient.importAccount(accountAsBytes);
        };

        reader.readAsArrayBuffer(file);
    }
});
```

### Transactions
You can use the WebClient to facilitate transactions between accounts.

Let's mint some tokens for our wallet from our faucet:
```typescript
const webClient = new WebClient();
await webClient.create_client();
const walletId = await webClient.new_wallet("OffChain", true);
const faucetId = await webClient.new_faucet("OffChain", true, "TOK", 6, 1_000_000);

// Syncs web client with node state.
await webClient.sync_state();
// Caches faucet account auth. A workaround to allow for synchronicity in the transaction flow.
await webClient.fetch_and_cache_account_auth_by_pub_key(faucetId);

/**
 * Mints 10_000 tokens for the previously created wallet via a Private Note and returns a transaction the following result object:
 * {
 *   transaction_id: string
 *   created_note_ids: string[]
 * }
 */
const newTxnResult = await webClient.new_mint_transaction(walletId, faucetId, "Private", 10_000);
console.log(newTxnResult.created_note_ids); // Prints the list of note ids created from this transaction

// Sync state again
await webClient.sync_state();

/**
 * Gets all of your existing transactions
 * Returns string[] of transaction ids
 */
const transactions = await webClient.get_transactions()
```

### Notes
You can use the WebClient to query for existing notes, export notes, and import notes

Here is an example of how to import a note from a file (generated, say, from the faucet at https://testnet.miden.io/ for a given account). This code exposes a simple button on an HTML page for a user to select a file. A listener is setup to capture this event, serialize the note file, and import it.
```typescript
let webClient = await createMidenWebClient();
let walletAccount = await webClient.new_wallet("OffChain", true);
console.log(walletAccount); // Prints the id that can be used to plug in to the deployed Miden faucet

<label for="noteFileInput" class="custom-file-upload">
    Choose Note File
</label>
<input type="file" id="noteFileInput" style="display: none;">
document.getElementById('noteFileInput').addEventListener('change', async function(event) {
    const file = event.target.files[0];
    if (file) {
        const reader = new FileReader();

        reader.onload = async function(e) {
            let webClient = await createMidenWebClient();

            const arrayBuffer = e.target.result;
            const byteArray = new Uint8Array(arrayBuffer);

            await webClient.import_note(byteArray, true); // imports the file generated from the faucet previously
        };

        reader.readAsArrayBuffer(file);
    }
});
```

Example of exporting a note:
```typescript
console.log("testExportNote started");

let webClient = await createMidenWebClient();

// Create a faucet and mint a mint transaction
let faucetId = await createNewFaucet(webClient, "OffChain", false, "DEN", "10", "1000000");
await syncState(webClient);
await new Promise(r => setTimeout(r, 20000)); // Artificial delays to ensure sync is processed on remote node before continuing 

await webClient.fetch_and_cache_account_auth_by_pub_key(faucetId);

let mintTransactionResult = await createNewMintTransaction(
    webClient,
    "0x9186b96f559e852f", // Insert target account id here
    faucetId,
    "Private",
    "1000"
);
await new Promise(r => setTimeout(r, 20000));
await syncState(webClient);

// Take the note created from the mint transaction, serialize it, and download it via the browser immediately
let result = await exportNote(webClient, mintTransactionResult.created_note_ids[0]);

const blob = new Blob([result], {type: 'application/octet-stream'});

// Create a URL for the Blob
const url = URL.createObjectURL(blob);

// Create a temporary anchor element
const a = document.createElement('a');
a.href = url;
a.download = 'exportNoteTest.mno'; // Specify the file name

// Append the anchor to the document
document.body.appendChild(a);

// Programmatically click the anchor to trigger the download
a.click();

// Remove the anchor from the document
document.body.removeChild(a);

// Revoke the object URL to free up resources
URL.revokeObjectURL(url);
```

Get All Input Notes Example:
```typescript
let webClient = await createMidenWebClient();

/**
 * get_input_notes takes a filter to retrieve notes based on a specific status. The options are the following:
 * "All"
 * "Consumed"
 * "Committed"
 * "Expected"
 * "Processing"
 */
const notes = await webClient.get_input_notes("All")
```

## API Reference

```typescript
/**
 * @returns {Promise<SerializedAccountHeader>}
 * 
 * Example of returned object:
 * {
 *   id: string,
 *   nonce: string,
 *   vault_root: string,
 *   storage_root: string,
 *   code_root: string
 * }
 */
get_accounts(): Promise<SerializedAccountHeader>;

/**
 * @param {string} account_id
 * @returns {Promise<any>}
 */
get_account(account_id: string): Promise<any>;

/**
 * @param {any} pub_key_bytes
 * @returns {any}
 */
get_account_auth_by_pub_key(pub_key_bytes: any): any;

/**
 * @param {string} account_id
 * @returns {Promise<any>}
 */
fetch_and_cache_account_auth_by_pub_key(account_id: string): Promise<any>;

/**
 * @param {string} note_id
 * @param {string} export_type
 * @returns {Promise<any>}
 * 
 * export_type can be any of the following:
 * 
 * "Full"
 * "Partial"
 * "Id"
 */
export_note(note_id: string, export_type: string): Promise<any>;

/**
 * @param {any} account_bytes
 * @returns created account id as {Promise<string>}
 * 
 */
import_account(account_bytes: any): Promise<string>;

/**
 * @param {string} note_bytes
 * @param {boolean} verify
 * @returns {Promise<any>}
 */
import_note(note_bytes: string, verify: boolean): Promise<any>;

/**
 * @param {string} storage_type
 * @param {boolean} mutable
 * @returns {Promise<any>}
 */
new_wallet(storage_type: string, mutable: boolean): Promise<any>;

/**
 * @param {string} storage_type
 * @param {boolean} non_fungible
 * @param {string} token_symbol
 * @param {string} decimals
 * @param {string} max_supply
 * @returns {Promise<any>}
 */
new_faucet(storage_type: string, non_fungible: boolean, token_symbol: string, decimals: string, max_supply: string): Promise<any>;

/**
 * @param {string} target_account_id
 * @param {string} faucet_id
 * @param {string} note_type
 * @param {string} amount
 * @returns {Promise<NewTransactionResult>}
 * 
 * Example of a NewTransactionResult object:
 * {
 *   transaction_id: string,
 *   created_note_ids: string[]
 * }
 */
new_mint_transaction(target_account_id: string, faucet_id: string, note_type: string, amount: string): Promise<NewTransactionResult>;

/**
 * @param {string} sender_account_id
 * @param {string} target_account_id
 * @param {string} faucet_id
 * @param {string} note_type
 * @param {string} amount
 * @param {string | undefined} [recall_height]
 * @returns {Promise<NewTransactionResult>}
 * 
 * Example of a NewTransactionResult object:
 * {
 *   transaction_id: string,
 *   created_note_ids: string[]
 * }
 */
new_send_transaction(sender_account_id: string, target_account_id: string, faucet_id: string, note_type: string, amount: string, recall_height?: string): Promise<NewTransactionResult>;

/**
 * @param {string} account_id
 * @param {(string)[]} list_of_notes
 * @returns {Promise<NewTransactionResult>}
 * 
 * Example of a NewTransactionResult object:
 * {
 *   transaction_id: string,
 *   created_note_ids: string[]
 * }
 */
new_consume_transaction(account_id: string, list_of_notes: (string)[]): Promise<NewTransactionResult>;

/**
 * @param {string} sender_account_id
 * @param {string} offered_asset_faucet_id
 * @param {string} offered_asset_amount
 * @param {string} requested_asset_faucet_id
 * @param {string} requested_asset_amount
 * @param {string} note_type
 * @returns {Promise<NewSwapTransactionResult>}
 * 
 * Example of a NewSwapTransactionResult object:
 * {
 *   transaction_id: string,
 *   expected_output_note_ids: string[],
 *.  expected_partial_note_ids: string[],
 *   payback_note_tag: string,
 * }
 */
new_swap_transaction(sender_account_id: string, offered_asset_faucet_id: string, offered_asset_amount: string, requested_asset_faucet_id: string, requested_asset_amount: string, note_type: string): Promise<NewSwapTransactionResult>;

/**
 * @param {any} filter
 * @returns {Promise<any>}
 * 
 * Examples of valid filters:
 * "All"
 * "Consumed"
 * "Committed"
 * "Expected"
 * "Processing"
 */
get_input_notes(filter: any): Promise<any>;

/**
 * @param {string} note_id
 * @returns note id as {Promise<any>}
 */
get_input_note(note_id: string): Promise<any>;

/**
 * @param {any} filter
 * @returns {Promise<any>}
 */
get_output_notes(filter: any): Promise<any>;

/**
 * @param {string} note_id
 * @returns {Promise<any>}
 * 
 * Examples of valid filters:
 * "All"
 * "Consumed"
 * "Committed"
 * "Expected"
 * "Processing"
 */
get_output_note(note_id: string): Promise<any>;

/**
 * @returns block number of latest block you synced to {Promise<any>}
 */
sync_state(): Promise<any>;

/**
 * @returns list of existing transaction ids {Promise<string[]>}
 */
get_transactions(): Promise<string[]>;

/**
 * @param {string} tag
 * @returns {Promise<any>}
 */
add_tag(tag: string): Promise<any>;

/**
 */
constructor();

/**
 * @param {string | undefined} [node_url]
 * @returns {Promise<any>}
 */
create_client(node_url?: string): Promise<any>;
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
