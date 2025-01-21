import * as chai from "chai";
import chaiAsPromised from "chai-as-promised";
import puppeteer from "puppeteer";
import { spawn } from "child_process";

import { register } from "ts-node";
import { env } from "process";

chai.use(chaiAsPromised);

register({
  project: "./tsconfig.json",
});

let serverProcess;
let browser;
let testingPage;

const TEST_SERVER_PORT = 8080;
const TEST_SERVER = `http://localhost:${TEST_SERVER_PORT}`;

// Should be in sync with the rpc port in tests/config/miden-node.toml
const LOCAL_MIDEN_NODE_PORT = 57291;
const REMOTE_TX_PROVER_PORT = 50051;

before(async () => {
  console.log("Starting test server...");
  serverProcess = spawn("http-server", ["./dist", "-p", TEST_SERVER_PORT], {
    stdio: "inherit",
    shell: process.platform == "win32",
  });

  try {
    browser = await puppeteer.launch({
      headless: true,
      protocolTimeout: 360000,
    });
    testingPage = await browser.newPage();
    await testingPage.goto(TEST_SERVER);
  } catch (error) {
    console.error("Failed to launch Puppeteer:", error);
    if (serverProcess && !serverProcess.killed) {
      serverProcess.kill("SIGTERM");
    }
    throw error;
  }

  testingPage.on("console", (msg) => console.log("PAGE LOG:", msg.text()));

  // Creates the client in the test context and attach to window object
  await testingPage.evaluate(
    async (rpc_port, remote_prover_port) => {
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
        Note,
        NoteAssets,
        NoteConsumability,
        NoteExecutionHint,
        NoteExecutionMode,
        NoteFilter,
        NoteFilterTypes,
        NoteIdAndArgs,
        NoteIdAndArgsArray,
        NoteInputs,
        NoteMetadata,
        NoteRecipient,
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
        WebClient,
      } = await import("./index.js");
      let rpc_url = `http://localhost:${rpc_port}`;
      let prover_url = null;
      if (remote_prover_port) {
        prover_url = `http://localhost:${remote_prover_port}`;
      }
      const client = new WebClient();
      await client.create_client(rpc_url, prover_url);

      window.client = client;
      window.Account = Account;
      window.AccountHeader = AccountHeader;
      window.AccountId = AccountId;
      window.AccountStorageMode = AccountStorageMode;
      window.AdviceMap = AdviceMap;
      window.AuthSecretKey = AuthSecretKey;
      window.ConsumableNoteRecord = ConsumableNoteRecord;
      window.Felt = Felt;
      window.FeltArray = FeltArray;
      window.FungibleAsset = FungibleAsset;
      window.Note = Note;
      window.NoteAssets = NoteAssets;
      window.NoteConsumability = NoteConsumability;
      window.NoteExecutionHint = NoteExecutionHint;
      window.NoteExecutionMode = NoteExecutionMode;
      window.NoteFilter = NoteFilter;
      window.NoteFilterTypes = NoteFilterTypes;
      window.NoteIdAndArgs = NoteIdAndArgs;
      window.NoteIdAndArgsArray = NoteIdAndArgsArray;
      window.NoteInputs = NoteInputs;
      window.NoteMetadata = NoteMetadata;
      window.NoteRecipient = NoteRecipient;
      window.NoteTag = NoteTag;
      window.NoteType = NoteType;
      window.OutputNote = OutputNote;
      window.OutputNotesArray = OutputNotesArray;
      window.Rpo256 = Rpo256;
      window.TestUtils = TestUtils;
      window.TransactionFilter = TransactionFilter;
      window.TransactionProver = TransactionProver;
      window.TransactionRequest = TransactionRequest;
      window.TransactionRequestBuilder = TransactionRequestBuilder;
      window.TransactionScriptInputPair = TransactionScriptInputPair;
      window.TransactionScriptInputPairArray = TransactionScriptInputPairArray;
      window.Word = Word;

      // Create a namespace for helper functions
      window.helpers = window.helpers || {};

      // Add the remote prover url to window
      window.remote_prover_url = prover_url;

      window.helpers.waitForTransaction = async (
        transactionId,
        maxWaitTime = 20000,
        delayInterval = 1000
      ) => {
        const client = window.client;
        let timeWaited = 0;
        while (true) {
          if (timeWaited >= maxWaitTime) {
            throw new Error("Timeout waiting for transaction");
          }
          await client.sync_state();
          const uncomittedTransactions = await client.get_transactions(
            window.TransactionFilter.uncomitted()
          );
          let uncomittedTransactionIds = uncomittedTransactions.map(
            (transaction) => transaction.id().to_hex()
          );
          if (!uncomittedTransactionIds.includes(transactionId)) {
            break;
          }
          await new Promise((r) => setTimeout(r, delayInterval));
          timeWaited += delayInterval;
        }
      };
    },
    LOCAL_MIDEN_NODE_PORT,
    env.REMOTE_PROVER ? REMOTE_TX_PROVER_PORT : null
  );
});

beforeEach(async () => {
  await testingPage.evaluate(async () => {
    // Open a connection to the list of databases
    const databases = await indexedDB.databases();
    for (const db of databases) {
      // Delete each database by name
      indexedDB.deleteDatabase(db.name);
    }
  });
});

after(async () => {
  console.log("Stopping test server...");

  console.log("Closing browser...");
  await browser.close();
  console.log("Browser closed.");

  console.log("Beginning server process cleanup...");
  if (serverProcess && !serverProcess.killed) {
    console.log("Killing server process...");
    serverProcess.kill("SIGTERM"); // Send the SIGTERM signal to terminate the server
  }

  console.log("Waiting for server process to exit...");
  await new Promise((resolve, reject) => {
    if (serverProcess.exitCode !== null) {
      // Process has already exited, resolve immediately
      console.log(
        `Server process had already exited with code ${serverProcess.exitCode}`
      );
      return resolve();
    }

    serverProcess.on("close", (code) => {
      console.log(`Server process exited with code ${code}`);
      resolve();
    });

    serverProcess.on("error", (error) => {
      console.error("Error killing server process:", error);
      reject(error);
    });
  });

  console.log("Test server stopped.");
});

export { testingPage };
