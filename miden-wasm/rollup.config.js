import rust from "@wasm-tool/rollup-plugin-rust";
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";

export default [
    {
        input: {
            wasm: "./js/wasm.js",
        },
        output: {
            dir: `dist`,
            format: "es",
            sourcemap: true,
            assetFileNames: "assets/[name][extname]",
        },
        plugins: [
            rust({
                cargoArgs: [
                    "--features", "wasm",
                    "--config", `build.rustflags=["-C", "target-feature=+atomics,+bulk-memory,+mutable-globals", "-C", "link-arg=--max-memory=4294967296"]`,
                    // "--no-default-features",
                    // "-Z", "build-std=panic_abort,std",
                ],

                experimental: {
                    typescriptDeclarationDir: "dist/crates",
                },
            }),
            resolve(),
            commonjs(),
        ],
    },
    {
        input: {
            index: "./js/index.js",
        },
        output: {
            dir: `dist`,
            format: "es",
            sourcemap: true,
        },
        plugins: [
            resolve(), // Ensure this resolves node modules
            commonjs(), // Convert CommonJS modules to ES6
        ],
    }
];
