import * as chai from "chai";
import chaiAsPromised from "chai-as-promised";
import puppeteer from "puppeteer";
import { spawn } from "child_process";

import { register } from "ts-node";
import { env } from "process";
import { clearStore } from "./webClientTestUtils.js";

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

  testingPage.on("pageerror", (err) => {
    console.error("PAGE ERROR:", err);
  });

  testingPage.on("error", (err) => {
    console.error("PUPPETEER ERROR:", err);
  });

  // Creates the client in the test context and attach to window object
  await testingPage.evaluate(
    async (rpcPort, remoteProverPort) => {
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
        WebClient,
      } = await import("./index.js");
      let rpcUrl = `http://localhost:${rpcPort}`;
      let proverUrl = null;
      if (remoteProverPort) {
        proverUrl = `http://localhost:${remoteProverPort}`;
      }
      const client = await WebClient.createClient(rpcUrl);

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
      window.NoteScript = NoteScript;
      window.NoteTag = NoteTag;
      window.NoteType = NoteType;
      window.OutputNote = OutputNote;
      window.OutputNotesArray = OutputNotesArray;
      window.Rpo256 = Rpo256;
      window.TestUtils = TestUtils;
      window.TransactionFilter = TransactionFilter;
      window.TransactionProver = TransactionProver;
      window.TransactionRequest = TransactionRequest;
      window.TransactionResult = TransactionResult;
      window.TransactionRequestBuilder = TransactionRequestBuilder;
      window.TransactionScriptInputPair = TransactionScriptInputPair;
      window.TransactionScriptInputPairArray = TransactionScriptInputPairArray;
      window.WebClient = WebClient;
      window.Word = Word;

      // Create a namespace for helper functions
      window.helpers = window.helpers || {};

      // Add the remote prover url to window
      window.remoteProverUrl = proverUrl;
      if (window.remoteProverUrl) {
        window.remoteProverInstance = window.TransactionProver.newRemoteProver(
          window.remoteProverUrl
        );
      }

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
          await client.syncState();
          const uncommittedTransactions = await client.getTransactions(
            window.TransactionFilter.uncommitted()
          );
          let uncommittedTransactionIds = uncommittedTransactions.map(
            (transaction) => transaction.id().toHex()
          );
          if (!uncommittedTransactionIds.includes(transactionId)) {
            break;
          }
          await new Promise((r) => setTimeout(r, delayInterval));
          timeWaited += delayInterval;
        }
      };

      window.helpers.refreshClient = async (initSeed) => {
        const client = await WebClient.createClient(rpcUrl, initSeed);
        window.client = client;
      };
    },
    LOCAL_MIDEN_NODE_PORT,
    env.REMOTE_PROVER ? REMOTE_TX_PROVER_PORT : null
  );
});

beforeEach(async () => {
  await clearStore();
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
