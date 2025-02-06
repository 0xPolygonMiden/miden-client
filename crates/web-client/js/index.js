import wasm from "../dist/wasm.js";

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
  Word
};

// Wrapper for WebClient
export class WebClient {
  constructor(rpcUrl, proverUrl, seed) {
    this.rpcUrl = rpcUrl;
    this.proverUrl = proverUrl;
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

    // Create a promise that resolves when the worker signals it is ready.
    this.ready = new Promise((resolve) => {
      this.readyResolver = resolve;
    });

    // Listen for messages from the worker.
    this.worker.addEventListener("message", (event) => {
      const data = event.data;

      // Worker initialization message.
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
          console.error(`WebClient: Error from worker in ${methodName}:`, error);
          reject(new Error(error));
        } else {
          resolve(result);
        }
      }
    });

    // Once ready, initialize the worker.
    this.ready.then(() => {
      this.worker.postMessage({ action: "init", args: [this.rpcUrl, this.proverUrl, this.seed] });
    });

    // Create the underlying WASM WebClient.
    this.wasmWebClient = new WasmWebClient();
  }

  /**
   * Factory method to create and initialize a WebClient instance.
   * This method is async so you can await the asynchronous call to create_client().
   *
   * @param {string} rpcUrl - The RPC URL.
   * @param {string} proverUrl - The prover URL.
   * @param {string} seed - The seed for the account.
   * @returns {Promise<WebClient>} The fully initialized WebClient.
   */
  static async create_client(rpcUrl, proverUrl, seed) {
    // Construct the instance (synchronously).
    const instance = new WebClient(rpcUrl, proverUrl, seed);

    // Wait for the underlying wasmWebClient to be initialized.
    await instance.wasmWebClient.create_client(rpcUrl, proverUrl, seed);

    // Optionally, you might also want to wait until the worker is ready,
    // for example:
    // await instance.ready;

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
      this.worker.postMessage({ action: "callMethod", methodName, args, requestId });
    });
  }

  /**
   * Call a method directly on the WASM WebClient.
   * @param {string} methodName - Name of the method to call.
   * @param  {...any} args - Arguments for the method.
   * @returns {Promise<any>}
   */
  async callMethodDirectly(methodName, ...args) {
    if (!this.wasmWebClient) {
      throw new Error("WASM WebClient is not initialized.");
    }
    const method = this.wasmWebClient[methodName];
    if (typeof method !== "function") {
      throw new Error(`Method ${methodName} does not exist on WASM WebClient.`);
    }
    return await method.apply(this.wasmWebClient, args);
  }

  // ----- Explicitly Wrapped Methods (Worker-Forwarded) -----

  async new_wallet(storageMode, mutable) {
    try {
      const serializedStorageMode = storageMode.as_str();
      const serializedAccountBytes = await this.callMethodWithWorker("new_wallet", serializedStorageMode, mutable);
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
        "new_faucet",
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
    const serializedAccountId = accountId.to_string();
    const serializedTransactionRequest = transactionRequest.serialize();
    try {
      const serializedTransactionResultBytes = await this.callMethodWithWorker(
        "new_transaction",
        serializedAccountId,
        serializedTransactionRequest
      );
      return wasm.TransactionResult.deserialize(new Uint8Array(serializedTransactionResultBytes));
    } catch (error) {
      console.error("INDEX.JS: Error in new_transaction:", error);
      throw error;
    }
  }

  async new_mint_transaction(targetAccountId, faucetId, noteType, amount) {
    const serializedTargetAccountId = targetAccountId.to_string();
    const serializedFaucetId = faucetId.to_string();
    const serializedNoteType = noteType.as_str();
    const serializedAmount = amount.toString();
    try {
      const serializedTransactionResultBytes = await this.callMethodWithWorker(
        "new_mint_transaction",
        serializedTargetAccountId,
        serializedFaucetId,
        serializedNoteType,
        serializedAmount
      );
      return wasm.TransactionResult.deserialize(new Uint8Array(serializedTransactionResultBytes));
    } catch (error) {
      console.error("INDEX.JS: Error in new_mint_transaction:", error);
      throw error; // Ensure the test catches and asserts
    }
  }

  async new_consume_transaction(targetAccountId, noteId) {
    const serializedTargetAccountId = targetAccountId.to_string();
    try {
      const serializedTransactionResultBytes = await this.callMethodWithWorker(
        "new_consume_transaction",
        serializedTargetAccountId,
        noteId
      );
      return wasm.TransactionResult.deserialize(new Uint8Array(serializedTransactionResultBytes));
    } catch (error) {
      console.error("INDEX.JS: Error in consume_transaction:", JSON.stringify(error));
      throw error;
    }
  }

  async new_send_transaction(senderAccountId, receiverAccountId, faucetId, noteType, amount, recallHeight = null) {
    const serializedSenderAccountId = senderAccountId.to_string();
    const serializedReceiverAccountId = receiverAccountId.to_string();
    const serializedFaucetId = faucetId.to_string();
    const serializedNoteType = noteType.as_str();
    const serializedAmount = amount.toString();
    try {
      const serializedTransactionResultBytes = await this.callMethodWithWorker(
        "new_send_transaction",
        serializedSenderAccountId,
        serializedReceiverAccountId,
        serializedFaucetId,
        serializedNoteType,
        serializedAmount,
        recallHeight
      );
      return wasm.TransactionResult.deserialize(new Uint8Array(serializedTransactionResultBytes));
    } catch (error) {
      console.error("INDEX.JS: Error in send_transaction:", error);
      throw error;
    }
  }

  async submit_transaction(transactionResult) {
    const serializedTransactionResult = transactionResult.serialize();
    try {
      await this.callMethodWithWorker("submit_transaction", serializedTransactionResult);
    } catch (error) {
      console.error("INDEX.JS: Error in submit_transaction:", error);
      throw error;
    }
  }

  async submit_transaction_with_prover(transactionResult, prover) {
    const serializedTransactionResult = transactionResult.serialize();
    const serializedProver = prover.as_str();
    try {
      await this.callMethodWithWorker("submit_transaction_with_prover", serializedTransactionResult, serializedProver);
    } catch (error) {
      console.error("INDEX.JS: Error in submit_transaction_with_prover:", error);
      throw error;
    }
  }

  async sync_state() {
    try {
      await this.callMethodWithWorker("sync_state");
    } catch (error) {
      console.error("INDEX.JS: Error in sync_state:", error);
      throw error
    }
  }

  terminate() {
    this.worker.terminate();
  }
}
