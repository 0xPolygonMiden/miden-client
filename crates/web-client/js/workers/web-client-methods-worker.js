import wasm from "../../dist/wasm.js";

const WorkerAction = Object.freeze({
  INIT: "init",
  CALL_METHOD: "callMethod",
});

const MethodName = Object.freeze({
  CREATE_CLIENT: "create_client",
  NEW_WALLET: "new_wallet",
  NEW_FAUCET: "new_faucet",
  NEW_TRANSACTION: "new_transaction",
  NEW_MINT_TRANSACTION: "new_mint_transaction",
  NEW_CONSUME_TRANSACTION: "new_consume_transaction",
  NEW_SEND_TRANSACTION: "new_send_transaction",
  SUBMIT_TRANSACTION: "submit_transaction",
  SUBMIT_TRANSACTION_WITH_PROVER: "submit_transaction_with_prover",
  SYNC_STATE: "sync_state",
});

// Global state variables.
let wasmWebClient = null;
let ready = false; // Indicates if the worker is fully initialized.
let messageQueue = []; // Queue for sequential processing.
let processing = false; // Flag to ensure one message is processed at a time.

// Define a mapping from method names to handler functions.
const methodHandlers = {
  [MethodName.NEW_WALLET]: async (args) => {
    const [walletStorageModeStr, mutable] = args;
    const walletStorageMode = wasm.AccountStorageMode.from_str(walletStorageModeStr);
    const wallet = await wasmWebClient.new_wallet(walletStorageMode, mutable);
    const serializedWallet = await wallet.serialize();
    return serializedWallet.buffer;
  },
  [MethodName.NEW_FAUCET]: async (args) => {
    const [faucetStorageModeStr, nonFungible, tokenSymbol, decimals, maxSupplyStr] = args;
    const faucetStorageMode = wasm.AccountStorageMode.from_str(faucetStorageModeStr);
    const maxSupply = BigInt(maxSupplyStr);
    const faucet = await wasmWebClient.new_faucet(faucetStorageMode, nonFungible, tokenSymbol, decimals, maxSupply);
    const serializedFaucet = await faucet.serialize();
    return serializedFaucet.buffer;
  },
  [MethodName.NEW_TRANSACTION]: async (args) => {
    const [accountIdStr, serializedTransactionRequest] = args;
    const accountId = wasm.AccountId.from_hex(accountIdStr);
    const transactionRequest = wasm.TransactionRequest.deserialize(new Uint8Array(serializedTransactionRequest));
    await wasmWebClient.fetch_and_cache_account_auth_by_pub_key(accountId);
    const transactionResult = await wasmWebClient.new_transaction(accountId, transactionRequest);
    const serializedTransactionResult = await transactionResult.serialize();
    return serializedTransactionResult.buffer;
  },
  [MethodName.NEW_MINT_TRANSACTION]: async (args) => {
    const [targetAccountIdStr, faucetIdStr, noteTypeStr, amountStr] = args;
    const targetAccountId = wasm.AccountId.from_hex(targetAccountIdStr);
    const faucetId = wasm.AccountId.from_hex(faucetIdStr);
    const noteType = wasm.NoteType.from_str(noteTypeStr);
    const amount = BigInt(amountStr);
    await wasmWebClient.fetch_and_cache_account_auth_by_pub_key(faucetId);
    const transactionResult = await wasmWebClient.new_mint_transaction(targetAccountId, faucetId, noteType, amount);
    const serializedTransactionResult = await transactionResult.serialize();
    return serializedTransactionResult.buffer;
  },
  [MethodName.NEW_CONSUME_TRANSACTION]: async (args) => {
    const [targetAccountIdStr, noteId] = args;
    const targetAccountId = wasm.AccountId.from_hex(targetAccountIdStr);
    await wasmWebClient.fetch_and_cache_account_auth_by_pub_key(targetAccountId);
    const transactionResult = await wasmWebClient.new_consume_transaction(targetAccountId, noteId);
    const serializedTransactionResult = await transactionResult.serialize();
    return serializedTransactionResult.buffer;
  },
  [MethodName.NEW_SEND_TRANSACTION]: async (args) => {
    const [senderAccountIdStr, receiverAccountIdStr, faucetIdStr, noteTypeStr, amountStr, recallHeight] = args;
    const senderAccountId = wasm.AccountId.from_hex(senderAccountIdStr);
    const receiverAccountId = wasm.AccountId.from_hex(receiverAccountIdStr);
    const faucetId = wasm.AccountId.from_hex(faucetIdStr);
    const noteType = wasm.NoteType.from_str(noteTypeStr);
    const amount = BigInt(amountStr);
    await wasmWebClient.fetch_and_cache_account_auth_by_pub_key(senderAccountId);
    const transactionResult = await wasmWebClient.new_send_transaction(
      senderAccountId,
      receiverAccountId,
      faucetId,
      noteType,
      amount,
      recallHeight
    );
    const serializedTransactionResult = await transactionResult.serialize();
    return serializedTransactionResult.buffer;
  },
  [MethodName.SUBMIT_TRANSACTION]: async (args) => {
    const [serializedTransactionResult] = args;
    const transactionResult = wasm.TransactionResult.deserialize(new Uint8Array(serializedTransactionResult));
    await wasmWebClient.submit_transaction(transactionResult);
    return;
  },
  [MethodName.SUBMIT_TRANSACTION_WITH_PROVER]: async (args) => {
    const [serializedTransactionResult, serializedProver] = args;
    const transactionResult = wasm.TransactionResult.deserialize(new Uint8Array(serializedTransactionResult));
    let prover;
    if (serializedProver.startsWith("remote:")) {
        // For a remote prover, extract the endpoint.
        // For example, "remote:https://my-custom-endpoint.com" becomes "https://my-custom-endpoint.com"
        const endpoint = serializedProver.split("remote:")[1];
        prover = wasm.TransactionProver.from_str("remote", endpoint);
      } else if (serializedProver === "local") {
        prover = TransactionProver.from_str("local", null);
      } else {
        throw new Error("Invalid prover tag received in worker");
      }
    await wasmWebClient.submit_transaction_with_prover(transactionResult, prover);
  },
  [MethodName.SYNC_STATE]: async (args) => {
    await wasmWebClient.sync_state();
    return null;
  },
};

/**
 * Process a single message event.
 */
async function processMessage(event) {
  const { action, args, methodName, requestId } = event.data;
  try {
    if (action === WorkerAction.INIT) {
      const [rpcUrl, proverUrl, seed] = args;
      // Initialize the WASM WebClient.
      wasmWebClient = new wasm.WebClient();
      await wasmWebClient.create_client(rpcUrl, proverUrl, seed);
      ready = true;
      // Signal that the worker is fully initialized.
      self.postMessage({ ready: true });
      return;
    } else if (action === WorkerAction.CALL_METHOD) {
      if (!ready) {
        throw new Error("Worker is not ready. Please initialize first.");
      }
      if (!wasmWebClient) {
        throw new Error("WebClient not initialized in worker.");
      }
      // Look up the handler from the mapping.
      const handler = methodHandlers[methodName];
      if (!handler) {
        throw new Error(`Unsupported method: ${methodName}`);
      }
      const result = await handler(args);
      self.postMessage({ requestId, result });
      return;
    } else {
      throw new Error(`Unsupported action: ${action}`);
    }
  } catch (error) {
    console.error(`WORKER: Error occurred - ${error}`);
    self.postMessage({ requestId, error: error });
  }
}

/**
 * Process messages one at a time from the messageQueue.
 */
async function processQueue() {
  if (processing || messageQueue.length === 0) return;
  processing = true;
  const event = messageQueue.shift();
  try {
    await processMessage(event);
  } finally {
    processing = false;
    processQueue(); // Process next message in queue.
  }
}

// Enqueue incoming messages and process them sequentially.
self.onmessage = (event) => {
  messageQueue.push(event);
  processQueue();
};

// Immediately signal that the worker script has loaded.
// This tells the main thread that the file is fully loaded before sending the "init" message.
self.postMessage({ ready: true });
