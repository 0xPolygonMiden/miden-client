.DEFAULT_GOAL := help

.PHONY: help
help: ## Show description of all commands
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# --- Variables -----------------------------------------------------------------------------------

FEATURES_WEB_CLIENT=--features "testing"
FEATURES_CLIENT=--features "testing, concurrent"
FEATURES_CLI=--features "testing, concurrent"
NODE_FEATURES_TESTING=--features "testing"
WARNINGS=RUSTDOCFLAGS="-D warnings"
NODE_BRANCH="igamigo-fix-build"

# --- Linting -------------------------------------------------------------------------------------

.PHONY: clippy
 clippy: ## Runs Clippy with configs
	cargo +nightly clippy --workspace --exclude miden-client-web --all-targets $(FEATURES_CLI) -- -D warnings

.PHONY: clippy-wasm
 clippy-wasm: ## Runs Clippy for the miden-client-web package
	cargo +nightly clippy --package miden-client-web --target wasm32-unknown-unknown --all-targets $(FEATURES_WEB_CLIENT) -- -D warnings

.PHONY: fix
fix: ## Runs Fix with configs
	cargo +nightly fix --workspace --exclude miden-client-web --allow-staged --allow-dirty --all-targets $(FEATURES_CLI)

.PHONY: fix-wasm
fix-wasm: ## Runs Fix for the miden-client-web package
	cargo +nightly fix --package miden-client-web --target wasm32-unknown-unknown --allow-staged --allow-dirty --all-targets $(FEATURES_WEB_CLIENT)

.PHONY: format
format: ## Run format using nightly toolchain
	cargo +nightly fmt --all

.PHONY: format-check
format-check: ## Run format using nightly toolchain but only in check mode
	cargo +nightly fmt --all --check

.PHONY: lint
lint: format fix clippy fix-wasm clippy-wasm ## Runs all linting tasks at once (clippy, fixing, formatting)

# --- Documentation site --------------------------------------------------------------------------

.PHONY: doc-deps
doc-deps: ## Install dependencies to build and serve documentation site
	pip3 install -r scripts/docs_requirements.txt

.PHONY: doc-build
doc-build: doc-deps ## Build documentation site
	mkdocs build

.PHONY: doc-serve
doc-serve: doc-deps ## Serve documentation site
	mkdocs serve

# --- Rust documentation --------------------------------------------------------------------------

.PHONY: doc
doc: ## Generate & check rust documentation. You'll need `jq` in order for this to run.
	@cd crates/rust-client && \
	FEATURES=$$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "miden-client") | .features | keys[] | select(. != "web-tonic" and . != "idxdb")' | tr '\n' ',') && \
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features "$$FEATURES" --keep-going --release

# --- Testing -------------------------------------------------------------------------------------

.PHONY: test
test: ## Run tests
	cargo nextest run --workspace --exclude miden-client-web --release --lib $(FEATURES_CLIENT)

.PHONY: test-deps
test-deps: ## Install dependencies for tests
	cargo install cargo-nextest

# --- Integration testing -------------------------------------------------------------------------

.PHONY: integration-test
integration-test: ## Run integration tests
	cargo nextest run --workspace --exclude miden-client-web --release --test=integration $(FEATURES_CLI) --no-default-features

.PHONY: integration-test-full
integration-test-full: ## Run the integration test binary with ignored tests included
	cargo nextest run --workspace --exclude miden-client-web --release --test=integration $(FEATURES_CLI)
	cargo nextest run --workspace --exclude miden-client-web --release --test=integration $(FEATURES_CLI) --run-ignored ignored-only -- test_import_genesis_accounts_can_be_used_for_transactions

.PHONY: kill-node
kill-node: ## Kill node process
	pkill miden-node || echo 'process not running'

.PHONY: clean-node
clean-node: ## Clean node directory
	rm -rf miden-node

.PHONY: node
node: ## Setup node directory
	if [ -d miden-node ]; then cd miden-node ; else git clone https://github.com/0xPolygonMiden/miden-node.git && cd miden-node; fi
	cd miden-node && git checkout $(NODE_BRANCH) && git pull origin $(NODE_BRANCH) && cargo update
	cd miden-node && rm -rf miden-store.sqlite3*
	cd miden-node && cargo run --bin miden-node $(NODE_FEATURES_TESTING) -- make-genesis --inputs-path ../tests/config/genesis.toml --force

.PHONY: start-node
start-node: ## Run node. This requires the node repo to be present at `miden-node`
	cd miden-node && cargo run --bin miden-node $(NODE_FEATURES_TESTING) -- start --config ../tests/config/miden-node.toml node

# --- Installing ----------------------------------------------------------------------------------

install: ## Installs the CLI binary
	cargo install $(FEATURES_CLI) --path bin/miden-cli

# --- Building ------------------------------------------------------------------------------------

build: ## Builds the CLI binary and client library in release mode
	cargo build --workspace --exclude miden-client-web --release $(FEATURES_CLI)

build-wasm: ## Builds the client library for wasm32
	cargo build --package miden-client-web --target wasm32-unknown-unknown $(FEATURES_WEB_CLIENT)

# --- Check ---------------------------------------------------------------------------------------

.PHONY: check
check: ## Builds the CLI binary and client library in release mode
	cargo check --workspace --exclude miden-client-web --release $(FEATURES_CLI)

.PHONY: check-wasm
check-wasm: ## Builds the client library for wasm32
	cargo check --package miden-client-web --target wasm32-unknown-unknown $(FEATURES_WEB_CLIENT)
