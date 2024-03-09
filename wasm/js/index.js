import wasm from "../dist/wasm.js";

const {
    greet,
    WebClient
} = await wasm({
    importHook: () => {
        return new URL("assets/miden_wasm.wasm", import.meta.url);
    },
});

export {
    greet,
    WebClient,
};