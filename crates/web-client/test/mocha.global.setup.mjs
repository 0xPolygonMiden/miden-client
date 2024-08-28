import puppeteer from "puppeteer";
import { exec } from "child_process";

import { register } from "ts-node";

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
  serverProcess = exec(`http-server ./dist -p ${TEST_SERVER_PORT}`);
  browser = await puppeteer.launch({ headless: true });
  testingPage = await browser.newPage();
  await testingPage.goto(TEST_SERVER);

  // Uncomment below to enable console logging
  // testingPage.on("console", (msg) => console.log("PAGE LOG:", msg.text()));

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
  await browser.close();
  serverProcess.kill();
});

export { testingPage };
