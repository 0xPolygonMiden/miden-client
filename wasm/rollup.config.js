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
                    // This enables multi-threading
                    "--features", "testing", // Add this line to include the concurrent feature
                    "--config", `build.rustflags=["-C", "target-feature=+atomics,+bulk-memory,+mutable-globals", "-C", "link-arg=--max-memory=4294967296"]`,
                    "--no-default-features",
                    "-Z", "build-std=panic_abort,std",
                ],

                experimental: {
                    typescriptDeclarationDir: "dist/crates",
                },
            }),
            resolve(), // Add this
            commonjs(), // And this, if you have CommonJS modules
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
    }
];
