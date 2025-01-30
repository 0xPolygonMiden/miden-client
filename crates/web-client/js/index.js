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
 * Additionally, the wrapper provides a static create_client function. This static method
 * instantiates the WebClient object and ensures that the necessary create_client calls are
 * performed both in the main thread and within the worker thread. This dual initialization
 * correctly passes user parameters (RPC URL and seed) to both the main-thread
 * WASM WebClient and the worker-side instance.
 *
 * Because of this implementation, the only breaking change for end users is in the way the
 * web client is instantiated. Users should now use the WebClient.create_client static call.
 */
export class WebClient {
  constructor(rpcUrl, seed) {
    this.rpcUrl = rpcUrl;
    this.seed = seed;

    // Create the worker.
    this.worker = new Worker(
      new URL("./workers/web-client-methods-worker.js", import.meta.url),
      { type: "module" }
    );

    // Map to track pending worker requests.
    // Each key is a unique request ID and each value is an object
    // containing the corresponding promise's resolve and reject functions.
    this.pendingRequests = new Map();

    // Create a promise that resolves when the worker script has loaded.
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

      // Worker script loaded message.
      if (data.loaded) {
        this.loadedResolver();
        return;
      }

      // Worker initialization (ready) message.
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

    // Once the worker script has fully loaded, initialize the worker.
    this.loaded.then(() => {
      this.worker.postMessage({
        action: WorkerAction.INIT,
        args: [this.rpcUrl, this.seed],
      });
    });

    // Create the underlying WASM WebClient.
    this.wasmWebClient = new WasmWebClient();
  }

  /**
   * Factory method to create and initialize a WebClient instance.
   * This method is async so you can await the asynchronous call to create_client().
   *
   * @param {string} rpcUrl - The RPC URL.
   * @param {string} seed - The seed for the account.
   * @returns {Promise<WebClient>} The fully initialized WebClient.
   */
  static async create_client(rpcUrl, seed) {
    // Construct the instance (synchronously).
    const instance = new WebClient(rpcUrl, seed);

    // Wait for the underlying wasmWebClient to be initialized.
    await instance.wasmWebClient.create_client(rpcUrl, seed);

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

  async new_wallet(storageMode, mutable) {
    try {
      const serializedStorageMode = storageMode.as_str();
      const serializedAccountBytes = await this.callMethodWithWorker(
        MethodName.NEW_WALLET,
        serializedStorageMode,
        mutable
      );
      return wasm.Account.deserialize(new Uint8Array(serializedAccountBytes));
    } catch (error) {
      console.error("INDEX.JS: Error in new_wallet:", error);
      throw error;
    }
  }

  async new_faucet(storageMode, nonFungible, tokenSymbol, decimals, maxSupply) {
    try {
      const serializedStorageMode = storageMode.as_str();
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
      console.error("INDEX.JS: Error in new_faucet:", error);
      throw error;
    }
  }

  async new_transaction(accountId, transactionRequest) {
    try {
      const serializedAccountId = accountId.to_string();
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
      console.error("INDEX.JS: Error in new_transaction:", error);
      throw error;
    }
  }

  async new_mint_transaction(targetAccountId, faucetId, noteType, amount) {
    try {
      const serializedTargetAccountId = targetAccountId.to_string();
      const serializedFaucetId = faucetId.to_string();
      const serializedNoteType = noteType.serialize();
      const serializedAmount = amount.toString();
      const serializedTransactionResultBytes = await this.callMethodWithWorker(
        MethodName.NEW_MINT_TRANSACTION,
        serializedTargetAccountId,
        serializedFaucetId,
        serializedNoteType,
        serializedAmount
      );
      return wasm.TransactionResult.deserialize(
        new Uint8Array(serializedTransactionResultBytes)
      );
    } catch (error) {
      console.error("INDEX.JS: Error in new_mint_transaction:", error);
      throw error; // Ensure the test catches and asserts
    }
  }

  async new_consume_transaction(targetAccountId, noteId) {
    try {
      const serializedTargetAccountId = targetAccountId.to_string();
      const serializedTransactionResultBytes = await this.callMethodWithWorker(
        MethodName.NEW_CONSUME_TRANSACTION,
        serializedTargetAccountId,
        noteId
      );
      return wasm.TransactionResult.deserialize(
        new Uint8Array(serializedTransactionResultBytes)
      );
    } catch (error) {
      console.error(
        "INDEX.JS: Error in consume_transaction:",
        JSON.stringify(error)
      );
      throw error;
    }
  }

  async new_send_transaction(
    senderAccountId,
    receiverAccountId,
    faucetId,
    noteType,
    amount,
    recallHeight = null
  ) {
    try {
      const serializedSenderAccountId = senderAccountId.to_string();
      const serializedReceiverAccountId = receiverAccountId.to_string();
      const serializedFaucetId = faucetId.to_string();
      const serializedNoteType = noteType.serialize();
      const serializedAmount = amount.toString();
      const serializedTransactionResultBytes = await this.callMethodWithWorker(
        MethodName.NEW_SEND_TRANSACTION,
        serializedSenderAccountId,
        serializedReceiverAccountId,
        serializedFaucetId,
        serializedNoteType,
        serializedAmount,
        recallHeight
      );
      return wasm.TransactionResult.deserialize(
        new Uint8Array(serializedTransactionResultBytes)
      );
    } catch (error) {
      console.error("INDEX.JS: Error in send_transaction:", error);
      throw error;
    }
  }

  async submit_transaction(transactionResult, prover = null) {
    try {
      const serializedTransactionResult = transactionResult.serialize();
      const args = [serializedTransactionResult];

      // If a prover is provided, serialize it and add it to the args.
      if (prover) {
        args.push(prover.serialize());
      }

      // Always call the same worker method.
      await this.callMethodWithWorker(MethodName.SUBMIT_TRANSACTION, ...args);
    } catch (error) {
      console.error("INDEX.JS: Error in submit_transaction:", error);
      throw error;
    }
  }

  async sync_state() {
    try {
      const serializedSyncSummaryBytes = await this.callMethodWithWorker(
        MethodName.SYNC_STATE
      );
      return wasm.SyncSummary.deserialize(
        new Uint8Array(serializedSyncSummaryBytes)
      );
    } catch (error) {
      console.error("INDEX.JS: Error in sync_state:", error);
      throw error;
    }
  }

  terminate() {
    this.worker.terminate();
  }
}
