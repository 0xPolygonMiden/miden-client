export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export type InitOutput = typeof import("./miden_wasm");

export interface InitOptions {
    serverPath?: string;

    importHook?: (path: string) => InitInput | Promise<InitInput>;

    initializeHook?: (
        init: (path: InitInput | Promise<InitInput>, memory?: WebAssembly.Memory) => void,
        path: InitInput | Promise<InitInput>,
    ) => Promise<void>;
}

declare const init: (options?: InitOptions) => Promise<InitOutput>;
export default init;
