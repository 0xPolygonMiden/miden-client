import rust from "@wasm-tool/rollup-plugin-rust";
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import copy from "rollup-plugin-copy";

// Flag for testing builds
const testing = process.env.MIDEN_WEB_TESTING === "true";

export default [
  // Build the WASM module
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
      commonjs()
    ],
  },
  // Build the worker file
  {
    input: "./js/workers/web-client-methods-worker.js",
    output: {
      dir: `dist/workers`,
      format: "es",
      sourcemap: true,
    },
    plugins: [
      // rustPlugin, // Processes `wasm.js` imports
      resolve(),
      commonjs(),
      copy({
        targets: [
          // Copy WASM to `dist/workers/assets` for worker accessibility
          { src: "dist/assets/*.wasm", dest: "dist/workers/assets" },
        ],
        verbose: true,
      }),
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
    plugins: [
      resolve(),
      commonjs(),
    ],
  },
];
