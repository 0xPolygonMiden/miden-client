import wasm from "../dist/wasm.js";

const {
    WebClient
} = await wasm({
    importHook: () => {
        return new URL("assets/miden_client_web.wasm", import.meta.url); // the name before .wasm needs to match the package name in Cargo.toml
    },
});

export {
    WebClient,
};
