import wasm from "../dist/wasm.js";
import { MethodName, WorkerAction } from "./constants.js";

const {
  Account,
  AccountHeader,
  AccountId,
  AccountStorageMode,
  AdviceMap,
  AuthSecretKey,
  ConsumableNoteRecord,
  Felt,
  FeltArray,
  FungibleAsset,
  InputNoteState,
  Note,
  NoteAssets,
  NoteConsumability,
  NoteExecutionHint,
  NoteExecutionMode,
  NoteFilter,
  NoteFilterTypes,
  NoteId,
  NoteIdAndArgs,
  NoteIdAndArgsArray,
  NoteInputs,
  NoteMetadata,
  NoteRecipient,
  NoteScript,
  NoteTag,
  NoteType,
  OutputNote,
  OutputNotesArray,
  Rpo256,
  TestUtils,
  TransactionFilter,
  TransactionProver,
  TransactionRequest,
  TransactionResult,
  TransactionRequestBuilder,
  TransactionScriptInputPair,
  TransactionScriptInputPairArray,
  Word,
  WebClient: WasmWebClient, // Alias the WASM-exported WebClient
} = wasm;

export {
  Account,
  AccountHeader,
  AccountId,
  AccountStorageMode,
  AdviceMap,
  AuthSecretKey,
  ConsumableNoteRecord,
  Felt,
  FeltArray,
  FungibleAsset,
  InputNoteState,
  Note,
  NoteAssets,
  NoteConsumability,
  NoteExecutionHint,
  NoteExecutionMode,
  NoteFilter,
  NoteFilterTypes,
  NoteId,
  NoteIdAndArgs,
  NoteIdAndArgsArray,
  NoteInputs,
  NoteMetadata,
  NoteRecipient,
  NoteScript,
  NoteTag,
  NoteType,
  OutputNote,
  OutputNotesArray,
  Rpo256,
  TestUtils,
  TransactionFilter,
  TransactionProver,
  TransactionRequest,
  TransactionResult,
  TransactionRequestBuilder,
  TransactionScriptInputPair,
  TransactionScriptInputPairArray,
  Word,
};

/**
 * WebClient is a wrapper around the underlying WASM WebClient object.
 *
 * This wrapper serves several purposes:
 *
 * 1. It creates a dedicated web worker to offload computationally heavy tasks
 *    (such as creating accounts, executing transactions, submitting transactions, etc.)
 *    from the main thread, helping to prevent UI freezes in the browser.
 *
 * 2. It defines methods that mirror the API of the underlying WASM WebClient,
 *    with the intention of executing these functions via the web worker. This allows us
 *    to maintain the same API and parameters while benefiting from asynchronous, worker-based computation.
 *
 * 3. It employs a Proxy to forward any calls not designated for web worker computation
 *    directly to the underlying WASM WebClient instance.
 *
 * Additionally, the wrapper provides a static createClient function. This static method
 * instantiates the WebClient object and ensures that the necessary createClient calls are
 * performed both in the main thread and within the worker thread. This dual initialization
 * correctly passes user parameters (RPC URL and seed) to both the main-thread
 * WASM WebClient and the worker-side instance.
 *
 * Because of this implementation, the only breaking change for end users is in the way the
 * web client is instantiated. Users should now use the WebClient.createClient static call.
 */
export class WebClient {
  constructor(rpcUrl, seed) {
    this.rpcUrl = rpcUrl;
    this.seed = seed;

    // Check if Web Workers are available.
    if (false) {
      console.log("WebClient: Web Workers are available.");
      // Create the worker.
      this.worker = new Worker(
        new URL("./workers/web-client-methods-worker.js", import.meta.url),
        { type: "module" }
      );

      // Map to track pending worker requests.
      this.pendingRequests = new Map();

      // Promises to track when the worker script is loaded and ready.
      this.loaded = new Promise((resolve) => {
        this.loadedResolver = resolve;
      });

      // Create a promise that resolves when the worker signals that it is fully initialized.
      this.ready = new Promise((resolve) => {
        this.readyResolver = resolve;
      });

      // Listen for messages from the worker.
      this.worker.addEventListener("message", (event) => {
        const data = event.data;

        // Worker script loaded.
        if (data.loaded) {
          this.loadedResolver();
          return;
        }

        // Worker ready.
        if (data.ready) {
          this.readyResolver();
          return;
        }

        // Handle responses for method calls.
        const { requestId, error, result, methodName } = data;
        if (requestId && this.pendingRequests.has(requestId)) {
          const { resolve, reject } = this.pendingRequests.get(requestId);
          this.pendingRequests.delete(requestId);
          if (error) {
            console.error(
              `WebClient: Error from worker in ${methodName}:`,
              error
            );
            reject(new Error(error));
          } else {
            resolve(result);
          }
        }
      });

      // Once the worker script has loaded, initialize the worker.
      this.loaded.then(() => {
        this.worker.postMessage({
          action: WorkerAction.INIT,
          args: [this.rpcUrl, this.seed],
        });
      });
    } else {
      console.log("WebClient: Web Workers are not available.");
      // Worker not available; set up fallback values.
      this.worker = null;
      this.pendingRequests = null;
      this.loaded = Promise.resolve();
      this.ready = Promise.resolve();
    }

    // Create the underlying WASM WebClient.
    this.wasmWebClient = new WasmWebClient();
  }

  /**
   * Factory method to create and initialize a WebClient instance.
   * This method is async so you can await the asynchronous call to createClient().
   *
   * @param {string} rpcUrl - The RPC URL.
   * @param {string} seed - The seed for the account.
   * @returns {Promise<WebClient>} The fully initialized WebClient.
   */
  static async createClient(rpcUrl, seed) {
    // Construct the instance (synchronously).
    const instance = new WebClient(rpcUrl, seed);

    // Wait for the underlying wasmWebClient to be initialized.
    await instance.wasmWebClient.createClient(rpcUrl, seed);

    // Wait for the worker to be ready
    await instance.ready;

    // Return a proxy that forwards missing properties to wasmWebClient.
    return new Proxy(instance, {
      get(target, prop, receiver) {
        // If the property exists on the wrapper, return it.
        if (prop in target) {
          return Reflect.get(target, prop, receiver);
        }
        // Otherwise, if the wasmWebClient has it, return that.
        if (target.wasmWebClient && prop in target.wasmWebClient) {
          const value = target.wasmWebClient[prop];
          if (typeof value === "function") {
            return value.bind(target.wasmWebClient);
          }
          return value;
        }
        return undefined;
      },
    });
  }

  /**
   * Call a method via the worker.
   * @param {string} methodName - Name of the method to call.
   * @param  {...any} args - Arguments for the method.
   * @returns {Promise<any>}
   */
  async callMethodWithWorker(methodName, ...args) {
    await this.ready;
    // Create a unique request ID.
    const requestId = `${methodName}-${Date.now()}-${Math.random()}`;
    return new Promise((resolve, reject) => {
      // Save the resolve and reject callbacks in the pendingRequests map.
      this.pendingRequests.set(requestId, { resolve, reject });
      // Send the method call request to the worker.
      this.worker.postMessage({
        action: WorkerAction.CALL_METHOD,
        methodName,
        args,
        requestId,
      });
    });
  }

  // ----- Explicitly Wrapped Methods (Worker-Forwarded) -----

  async newWallet(storageMode, mutable, seed) {
    try {
      if (!this.worker) {
        return await this.wasmWebClient.newWallet(storageMode, mutable, seed);
      }
      const serializedStorageMode = storageMode.asStr();
      const serializedAccountBytes = await this.callMethodWithWorker(
        MethodName.NEW_WALLET,
        serializedStorageMode,
        mutable,
        seed
      );
      return wasm.Account.deserialize(new Uint8Array(serializedAccountBytes));
    } catch (error) {
      console.error("INDEX.JS: Error in newWallet:", error.toString());
      throw error;
    }
  }

  async newFaucet(storageMode, nonFungible, tokenSymbol, decimals, maxSupply) {
    try {
      if (!this.worker) {
        return await this.wasmWebClient.newFaucet(
          storageMode,
          nonFungible,
          tokenSymbol,
          decimals,
          maxSupply
        );
      }
      const serializedStorageMode = storageMode.asStr();
      const serializedMaxSupply = maxSupply.toString();
      const serializedAccountBytes = await this.callMethodWithWorker(
        MethodName.NEW_FAUCET,
        serializedStorageMode,
        nonFungible,
        tokenSymbol,
        decimals,
        serializedMaxSupply
      );

      return wasm.Account.deserialize(new Uint8Array(serializedAccountBytes));
    } catch (error) {
      console.error("INDEX.JS: Error in newFaucet:", error.toString());
      throw error;
    }
  }

  async newTransaction(accountId, transactionRequest) {
    try {
      if (!this.worker) {
        return await this.wasmWebClient.newTransaction(
          accountId,
          transactionRequest
        );
      }
      const serializedAccountId = accountId.toString();
      const serializedTransactionRequest = transactionRequest.serialize();
      const serializedTransactionResultBytes = await this.callMethodWithWorker(
        MethodName.NEW_TRANSACTION,
        serializedAccountId,
        serializedTransactionRequest
      );
      return wasm.TransactionResult.deserialize(
        new Uint8Array(serializedTransactionResultBytes)
      );
    } catch (error) {
      console.error("INDEX.JS: Error in newTransaction:", error.toString());
      throw error;
    }
  }

  async submitTransaction(transactionResult, prover = undefined) {
    try {
      if (!this.worker) {
        return await this.wasmWebClient.submitTransaction(
          transactionResult,
          prover
        );
      }
      const serializedTransactionResult = transactionResult.serialize();
      const args = [serializedTransactionResult];

      // If a prover is provided, serialize it and add it to the args.
      if (prover) {
        args.push(prover.serialize());
      }

      // Always call the same worker method.
      await this.callMethodWithWorker(MethodName.SUBMIT_TRANSACTION, ...args);
    } catch (error) {
      console.error("INDEX.JS: Error in submitTransaction:", error.toString());
      throw error;
    }
  }

  async syncState() {
    try {
      if (!this.worker) {
        return await this.wasmWebClient.syncState();
      }
      const serializedSyncSummaryBytes = await this.callMethodWithWorker(
        MethodName.SYNC_STATE
      );
      return wasm.SyncSummary.deserialize(
        new Uint8Array(serializedSyncSummaryBytes)
      );
    } catch (error) {
      console.error("INDEX.JS: Error in syncState:", error.toString());
      throw error;
    }
  }

  terminate() {
    this.worker.terminate();
  }
}
