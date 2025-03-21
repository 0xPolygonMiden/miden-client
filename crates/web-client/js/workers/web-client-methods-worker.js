import wasm from "../../dist/wasm.js";
import { MethodName, WorkerAction } from "../constants.js";

/**
 * Worker for executing WebClient methods in a separate thread.
 *
 * This worker offloads computationally heavy tasks from the main thread by handling
 * WebClient operations asynchronously. It imports the WASM module and instantiates a
 * WASM WebClient, then listens for messages from the main thread to perform one of two actions:
 *
 * 1. **Initialization (init):**
 *    - The worker receives an "init" message along with user parameters (RPC URL and seed).
 *    - It instantiates the WASM WebClient and calls its createClient method.
 *    - Once initialization is complete, the worker sends a `{ ready: true }` message back to signal
 *      that it is fully initialized.
 *
 * 2. **Method Invocation (callMethod):**
 *    - The worker receives a "callMethod" message with a specific method name and arguments.
 *    - It uses a mapping (defined in `methodHandlers`) to route the call to the corresponding WASM WebClient method.
 *    - Complex data is serialized before being sent and deserialized upon return.
 *    - The result (or any error) is then posted back to the main thread.
 *
 * The worker uses a message queue to process incoming messages sequentially, ensuring that only one message
 * is handled at a time.
 *
 * Additionally, the worker immediately sends a `{ loaded: true }` message upon script load. This informs the main
 * thread that the worker script is loaded and ready to receive the "init" message.
 *
 * Supported actions (defined in `WorkerAction`):
 *   - "init"       : Initialize the WASM WebClient with provided parameters.
 *   - "callMethod" : Invoke a designated method on the WASM WebClient.
 *
 * Supported method names are defined in the `MethodName` constant.
 */

// Global state variables.
let wasmWebClient = null;
let ready = false; // Indicates if the worker is fully initialized.
let messageQueue = []; // Queue for sequential processing.
let processing = false; // Flag to ensure one message is processed at a time.

// Define a mapping from method names to handler functions.
const methodHandlers = {
  [MethodName.NEW_WALLET]: async (args) => {
    const [walletStorageModeStr, mutable, seed] = args;
    const walletStorageMode =
      wasm.AccountStorageMode.tryFromStr(walletStorageModeStr);
    const wallet = await wasmWebClient.newWallet(
      walletStorageMode,
      mutable,
      seed
    );
    const serializedWallet = await wallet.serialize();
    return serializedWallet.buffer;
  },
  [MethodName.NEW_FAUCET]: async (args) => {
    const [
      faucetStorageModeStr,
      nonFungible,
      tokenSymbol,
      decimals,
      maxSupplyStr,
    ] = args;
    const faucetStorageMode =
      wasm.AccountStorageMode.tryFromStr(faucetStorageModeStr);
    const maxSupply = BigInt(maxSupplyStr);
    const faucet = await wasmWebClient.newFaucet(
      faucetStorageMode,
      nonFungible,
      tokenSymbol,
      decimals,
      maxSupply
    );
    const serializedFaucet = await faucet.serialize();
    return serializedFaucet.buffer;
  },
  [MethodName.NEW_TRANSACTION]: async (args) => {
    const [accountIdStr, serializedTransactionRequest] = args;
    const accountId = wasm.AccountId.fromHex(accountIdStr);
    const transactionRequest = wasm.TransactionRequest.deserialize(
      new Uint8Array(serializedTransactionRequest)
    );

    const transactionResult = await wasmWebClient.newTransaction(
      accountId,
      transactionRequest
    );
    const serializedTransactionResult = await transactionResult.serialize();
    return serializedTransactionResult.buffer;
  },
  [MethodName.NEW_MINT_TRANSACTION]: async (args) => {
    const [targetAccountIdStr, faucetIdStr, noteTypeBytes, amountStr] = args;
    const targetAccountId = wasm.AccountId.fromHex(targetAccountIdStr);
    const faucetId = wasm.AccountId.fromHex(faucetIdStr);
    const noteType = wasm.NoteType.deserialize(new Uint8Array(noteTypeBytes));
    const amount = BigInt(amountStr);

    const transactionResult = await wasmWebClient.newMintTransaction(
      targetAccountId,
      faucetId,
      noteType,
      amount
    );
    const serializedTransactionResult = await transactionResult.serialize();
    return serializedTransactionResult.buffer;
  },
  [MethodName.NEW_CONSUME_TRANSACTION]: async (args) => {
    const [targetAccountIdStr, noteId] = args;
    const targetAccountId = wasm.AccountId.fromHex(targetAccountIdStr);

    const transactionResult = await wasmWebClient.newConsumeTransaction(
      targetAccountId,
      noteId
    );
    const serializedTransactionResult = await transactionResult.serialize();
    return serializedTransactionResult.buffer;
  },
  [MethodName.NEW_SEND_TRANSACTION]: async (args) => {
    const [
      senderAccountIdStr,
      receiverAccountIdStr,
      faucetIdStr,
      noteTypeBytes,
      amountStr,
      recallHeight,
    ] = args;
    const senderAccountId = wasm.AccountId.fromHex(senderAccountIdStr);
    const receiverAccountId = wasm.AccountId.fromHex(receiverAccountIdStr);
    const faucetId = wasm.AccountId.fromHex(faucetIdStr);
    const noteType = wasm.NoteType.deserialize(new Uint8Array(noteTypeBytes));
    const amount = BigInt(amountStr);

    const transactionResult = await wasmWebClient.newSendTransaction(
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
    // Destructure the arguments. The prover may be undefined.
    const [serializedTransactionResult, serializedProver] = args;
    const transactionResult = wasm.TransactionResult.deserialize(
      new Uint8Array(serializedTransactionResult)
    );

    let prover = undefined;
    if (serializedProver) {
      if (serializedProver.startsWith("remote:")) {
        // For a remote prover, extract the endpoint.
        // For example, "remote:https://my-custom-endpoint.com" becomes "https://my-custom-endpoint.com"
        const endpoint = serializedProver.split("remote:")[1];
        prover = wasm.TransactionProver.deserialize("remote", endpoint);
      } else if (serializedProver === "local") {
        prover = wasm.TransactionProver.deserialize("local");
      } else {
        throw new Error("Invalid prover tag received in worker");
      }
    }

    // Call the unified submit_transaction method with an optional prover.
    await wasmWebClient.submitTransaction(transactionResult, prover);
    return;
  },
  [MethodName.SYNC_STATE]: async () => {
    const syncSummary = await wasmWebClient.syncState();
    const serializedSyncSummary = await syncSummary.serialize();
    return serializedSyncSummary.buffer;
  },
};

/**
 * Process a single message event.
 */
async function processMessage(event) {
  const { action, args, methodName, requestId } = event.data;
  try {
    if (action === WorkerAction.INIT) {
      const [rpcUrl, seed] = args;
      // Initialize the WASM WebClient.
      wasmWebClient = new wasm.WebClient();
      await wasmWebClient.createClient(rpcUrl, seed);
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
self.postMessage({ loaded: true });
