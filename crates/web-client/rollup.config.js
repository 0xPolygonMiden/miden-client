import rust from "@wasm-tool/rollup-plugin-rust";
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";

// Flag that indicates if the build is meant for testing purposes.
const testing = process.env.MIDEN_WEB_TESTING === "true";

/**
 * Rollup configuration file for building a Cargo project and creating a WebAssembly (WASM) module,
 * as well as bundling a dedicated web worker file.
 *
 * The configuration sets up three build processes:
 *
 * 1. **WASM Module Build:**
 *    Compiles Rust code into WASM using the @wasm-tool/rollup-plugin-rust plugin. This process
 *    applies specific cargo arguments to enable necessary WebAssembly features (such as atomics,
 *    bulk memory operations, and mutable globals) and to set maximum memory limits. For testing builds,
 *    the WASM optimization level is set to 0 to improve build times, reducing the feedback loop during development.
 *
 * 2. **Worker Build:**
 *    Bundles the dedicated web worker file (`web-client-methods-worker.js`) into the `dist/workers` directory.
 *    This configuration resolves WASM module imports and uses the copy plugin to ensure that the generated
 *    WASM assets are available to the worker.
 *
 * 3. **Main Entry Point Build:**
 *    Resolves and bundles the main JavaScript file (`index.js`) for the primary entry point of the application
 *    into the `dist` directory.
 *
 * Each build configuration outputs ES module format files with source maps to facilitate easier debugging.
 */
export default [
  {
    input: "./js/wasm.js",
    output: {
      dir: `dist`,
      format: "es",
      sourcemap: true,
      assetFileNames: "assets/[name][extname]",
    },
    plugins: [
      rust({
        extraArgs: {
          cargo: [
            "--features",
            "testing",
            "--config",
            `build.rustflags=["-C", "target-feature=+atomics,+bulk-memory,+mutable-globals", "-C", "link-arg=--max-memory=4294967296"]`,
            "--no-default-features",
          ],
          wasmOpt: testing
            ? ["-O0", "--enable-threads", "--enable-bulk-memory-opt"]
            : ["--enable-threads", "--enable-bulk-memory-opt"],
        },
        experimental: {
          typescriptDeclarationDir: "dist/crates",
        },
      }),
      resolve(),
      commonjs(),
    ],
  },
  // Build the main entry point
  {
    input: "./js/index.js",
    output: {
      dir: `dist`,
      format: "es",
      sourcemap: true,
    },
    plugins: [resolve(), commonjs()],
  },
];
