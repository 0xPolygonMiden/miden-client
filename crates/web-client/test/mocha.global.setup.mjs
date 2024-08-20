import puppeteer from "puppeteer";
import { exec } from "child_process";

import { register } from "ts-node";

register({
  project: "./tsconfig.json",
});

let serverProcess;
let browser;
let testingPage;

const LOCAL_SERVER_PORT = 8080;
const LOCAL_SERVER = `http://localhost:${LOCAL_SERVER_PORT}`;

before(async function () {
  console.log("Starting test server...");
  serverProcess = exec(`http-server ./dist -p ${LOCAL_SERVER_PORT}`);
  browser = await puppeteer.launch({ headless: true });
  testingPage = await browser.newPage();
  await testingPage.goto(LOCAL_SERVER);

  // Uncomment below to enable console logging
  // testingPage.on("console", (msg) => console.log("PAGE LOG:", msg.text()));

  // Creates the client in the test context and attach to window object
  await testingPage.exposeFunction("create_client", async () => {
    await testingPage.evaluate(async () => {
      const { WebClient } = await import("./index.js");
      // let rpc_url = "http://18.203.155.106:57291";
      let rpc_url = "http://localhost:57291";
      const client = new WebClient();
      await client.create_client(rpc_url);

      window.client = client;
    });
  });
});

after(async function () {
  console.log("Stopping test server...");
  await browser.close();
  serverProcess.kill();
});

export { testingPage };
