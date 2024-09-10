import { Page } from "puppeteer";

declare module "./mocha.global.setup.mjs" {
  export const testingPage: Page;
}
