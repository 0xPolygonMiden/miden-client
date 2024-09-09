import puppeteer from "puppeteer";
import { spawn } from "child_process";

import { register } from "ts-node";
import { env } from "process";

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

before(async () => {
  console.log("Starting test server...");
  serverProcess = spawn("http-server", ["./dist", "-p", TEST_SERVER_PORT], {
    stdio: "inherit",
  });

  browser = await puppeteer.launch({ headless: true });
  testingPage = await browser.newPage();
  await testingPage.goto(TEST_SERVER);

  if (env.DEBUG_MODE) {
    testingPage.on("console", (msg) => console.log("PAGE LOG:", msg.text()));
  }

  // Creates the client in the test context and attach to window object
  await testingPage.exposeFunction("create_client", async () => {
    await testingPage.evaluate(async (port) => {
      const { WebClient } = await import("./index.js");
      let rpc_url = `http://localhost:${port}`;
      const client = new WebClient();
      await client.create_client(rpc_url);

      window.client = client;
    }, LOCAL_MIDEN_NODE_PORT);
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
