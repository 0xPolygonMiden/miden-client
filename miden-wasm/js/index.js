import wasm from "../dist/wasm.js";

const {
    WebClient
} = await wasm({
    importHook: () => {
        return new URL("assets/miden_client.wasm", import.meta.url);
    },
});

export {
    WebClient,
};