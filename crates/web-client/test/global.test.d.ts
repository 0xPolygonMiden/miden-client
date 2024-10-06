import { Page } from "puppeteer";
import { Account, AccountHeader, AccountStorageMode, AuthSecretKey, TestUtils, WebClient } from "../dist/index";

declare global {
  interface Window {
    client: WebClient;
    Account: typeof Account;
    AccountHeader: typeof AccountHeader;
    AccountStorageMode: typeof AccountStorageMode;
    AuthSecretKey: typeof AuthSecretKey;
    TestUtils: typeof TestUtils;
    create_client: () => Promise<void>;
  }
}

declare module "./mocha.global.setup.mjs" {
  export const testingPage: Page;
}
