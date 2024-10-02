import rust from "@wasm-tool/rollup-plugin-rust";
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";

// Flag that indicates if the build is meant for testing purposes.
const testing = process.env.MIDEN_WEB_TESTING === 'true';

/**
 * Rollup configuration file for building a Cargo project and creating a WebAssembly (WASM) module.
 * The configuration sets up two build processes:
 * 1. Compiling Rust code into WASM using the @wasm-tool/rollup-plugin-rust plugin, with specific
 *    cargo arguments to enable WebAssembly features and set maximum memory limits. If the build is
 *    meant for testing, the WASM optimization level is set to 0 to improve build times, this is
 *    aimed at reducing the feedback loop during development.
 * 2. Resolving and bundling the generated WASM module along with the main JavaScript file
 *    (`index.js`) into the `dist` directory.
 *
 * The first configuration targets `wasm.js` to generate the WASM module, while the second
 * configuration targets `index.js` for the main entry point of the application.
 * Both configurations output ES module format files with source maps for easier debugging.
 */
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
                    "--features", "testing",
                    "--config", `build.rustflags=["-C", "target-feature=+atomics,+bulk-memory,+mutable-globals", "-C", "link-arg=--max-memory=4294967296"]`,
                    "--no-default-features",
                ],

                experimental: {
                    typescriptDeclarationDir: "dist/crates",
                },

                wasmOptArgs: testing ? ["-O0"] : null,
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
            resolve(),
            commonjs(),
        ],
    }
];
