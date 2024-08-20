# Testing

The .wasm must be run within the context of a webpage. To this end, we've set up a Mocha
test suite which hosts the .wasm on a local server and then executes WebClient commands
within the context of the web page.

## Running tests

1. In crates/web-client run `yarn test` to run all tests
2. For running an individual test by name run `yarn test -g <test-name>`

## Writing tests

1. The test setup in `mocha.global.setup.mjs` should expose the `create_client` function which can be used inside tests.
   - Any further setup of wasm code should be done in this file and similarly expose a function for testing here
2. `webClientTestUtils.js` should contain all interfaces for interacting with the web client. If further methods need to be added, follow existing patterns which use the exposed `testingPage` and pass through any required arguments to the page execution. Example:

```
/**
 *
 * @param {string} arg1
 * @param {boolean} arg2
 *
 * @returns {Promise<string>} The result
 */
export const webClientCall = async (arg1, arg2) => {
  return await testingPage.evaluate(
    async (_arg1, _arg2) => {
      if (!window.client) {
        await window.create_client();
      }

      /** @type {WebClient} */
      const client = window.client;
      const result = client.webClientCall(_arg1, _arg2);

      return result;
    },
    arg1,
    arg2
  );
};
```

- Add JSDocs to methods. This will allow typing in the `*.test.ts` files.
- We add the `if (!window.client)` to avoid spinning up clients unnecessarily lengthening test time. This unfortunately cannot be pulled out to a helper method as the testingPage scope does not share the scope of the file.
- Similarly, the boilerplate for passing args through as shown above is necessary due to scoping.

## Debugging

1. When inside of a `page.evaluate` , console logs are being sent to the servers console rather than your IDE's. You can uncomment the line as seen below in the `mocha.global.setup.mjs`:

```
    page.on("console", (msg) => console.log("PAGE LOG:", msg.text()));
```

This will forward logs from the server to your terminal logs
