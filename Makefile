.DEFAULT_GOAL := help

.PHONY: help
help: ## Show description of all commands
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# --- Variables -----------------------------------------------------------------------------------

# Enable file generation in the `src` directory.
# This is used in the build script of the client to generate the node RPC-related code, from the
# protobuf files.
CODEGEN=CODEGEN=1

FEATURES_WEB_CLIENT=--features "testing"
FEATURES_CLIENT=--features "testing, concurrent" --no-default-features
FEATURES_CLI=--features "concurrent"
WARNINGS=RUSTDOCFLAGS="-D warnings"

NODE_DIR="miden-node"
NODE_REPO="https://github.com/0xMiden/miden-node.git"
NODE_BRANCH="next"

PROVER_DIR="miden-base"
PROVER_REPO="https://github.com/0xMiden/miden-base.git"
PROVER_BRANCH="next"
PROVER_FEATURES_TESTING=--features "testing"
PROVER_PORT=50051

# --- Linting -------------------------------------------------------------------------------------

.PHONY: clippy
 clippy: ## Run Clippy with configs
	cargo clippy --workspace --exclude miden-client-web --all-targets $(FEATURES_CLI) -- -D warnings

.PHONY: clippy-wasm
 clippy-wasm: ## Run Clippy for the miden-client-web package
	cargo clippy --package miden-client-web --target wasm32-unknown-unknown --all-targets $(FEATURES_WEB_CLIENT) -- -D warnings

.PHONY: fix
fix: ## Run Fix with configs
	cargo +nightly fix --workspace --exclude miden-client-web --allow-staged --allow-dirty --all-targets $(FEATURES_CLI)

.PHONY: fix-wasm
fix-wasm: ## Run Fix for the miden-client-web package
	cargo +nightly fix --package miden-client-web --target wasm32-unknown-unknown --allow-staged --allow-dirty --all-targets $(FEATURES_WEB_CLIENT)

.PHONY: format
format: ## Run format using nightly toolchain
	cargo +nightly fmt --all && yarn prettier . --write && yarn eslint . --fix

.PHONY: format-check
format-check: ## Run format using nightly toolchain but only in check mode
	cargo +nightly fmt --all --check && yarn prettier . --check && yarn eslint .

.PHONY: lint
lint: format fix clippy fix-wasm clippy-wasm ## Run all linting tasks at once (clippy, fixing, formatting)

# --- Documentation --------------------------------------------------------------------------

.PHONY: doc
doc: ## Generate & check rust documentation. You'll need `jq` in order for this to run.
	@cd crates/rust-client && \
	FEATURES=$$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "miden-client") | .features | keys[] | select(. != "web-tonic" and . != "idxdb")' | tr '\n' ',') && \
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features "$$FEATURES" --keep-going --release

.PHONY: book
book: ## Builds the book & serves documentation site
	mdbook serve --open docs

# --- Testing -------------------------------------------------------------------------------------

.PHONY: test
test: ## Run tests
	$(CODEGEN) cargo nextest run --workspace --exclude miden-client-web --release --lib $(FEATURES_CLIENT)

.PHONY: test-deps
test-deps: ## Install dependencies for tests
	$(CODEGEN) cargo install cargo-nextest

.PHONY: test-docs
test-docs: ## Run documentation tests
	$(CODEGEN) cargo test --doc $(FEATURES_CLIENT)

# --- Integration testing -------------------------------------------------------------------------

.PHONY: integration-test
integration-test: ## Run integration tests
	$(CODEGEN) cargo nextest run --workspace --exclude miden-client-web --release --test=integration $(FEATURES_CLI) 

.PHONY: integration-test-web-client
integration-test-web-client: ## Run integration tests for the web client
	$(CODEGEN) cd ./crates/web-client && npm run test:clean

.PHONY: integration-test-remote-prover-web-client
integration-test-remote-prover-web-client: ## Run integration tests for the web client with remote prover
	$(CODEGEN) cd ./crates/web-client && npm run test:remote_prover

.PHONY: integration-test-full
integration-test-full: ## Run the integration test binary with ignored tests included
	$(CODEGEN) cargo nextest run --workspace --exclude miden-client-web --release --test=integration $(FEATURES_CLI)
	cargo nextest run --workspace --exclude miden-client-web --release --test=integration $(FEATURES_CLI) --run-ignored ignored-only -- test_import_genesis_accounts_can_be_used_for_transactions

.PHONY: kill-node
kill-node: ## Kill node process
	pkill miden-node || echo 'process not running'

.PHONY: clean-node
clean-node: ## Clean node directory
	rm -rf miden-node

.PHONY: node
node: setup-miden-node update-node-branch build-node ## Setup node directory

.PHONY: setup-miden-node
setup-miden-node: ## Clone the miden-node repository if it doesn't exist
	if [ ! -d $(NODE_DIR) ]; then git clone $(NODE_REPO) $(NODE_DIR); fi

.PHONY: update-node-branch
update-node-branch: setup-miden-base ## Checkout and update the specified branch in miden-node
	cd $(NODE_DIR) && git checkout $(NODE_BRANCH) && git pull origin $(NODE_BRANCH)

.PHONY: build-node
build-node: update-node-branch ## Update dependencies and build the node binary with specified features
	cd $(NODE_DIR) && rm -rf data accounts && mkdir data accounts && cargo run --locked --bin miden-node $(NODE_FEATURES_TESTING) --release -- bundled bootstrap --data-directory data --accounts-directory accounts

.PHONY: start-node
start-node: ## Run node. This requires the node repo to be present at `miden-node`
	cd $(NODE_DIR) && cargo run --bin miden-node $(NODE_FEATURES_TESTING) --release --locked -- bundled start --data-directory data --rpc.url http://localhost:57291

.PHONY: clean-prover
clean-prover: ## Uninstall prover
	cargo uninstall miden-proving-service || echo 'prover not installed'

.PHONY: prover
prover: setup-miden-base update-prover-branch build-prover ## Setup prover directory

.PHONY: setup-miden-base
setup-miden-base: ## Clone the miden-base repository if it doesn't exist
	if [ ! -d $(PROVER_DIR) ]; then git clone $(PROVER_REPO) $(PROVER_DIR); fi

.PHONY: update-prover-branch
update-prover-branch: setup-miden-base ## Checkout and update the specified branch in miden-base
	cd $(PROVER_DIR) && git checkout $(PROVER_BRANCH) && git pull origin $(PROVER_BRANCH)

.PHONY: build-prover
build-prover: update-prover-branch ## Build the prover binary with specified features
	cd $(PROVER_DIR) && cargo build --bin miden-proving-service --locked $(PROVER_FEATURES_TESTING) --release

.PHONY: start-prover
start-prover: ## Run prover. This requires the base repo to be present at `miden-base`
	cd $(PROVER_DIR) && RUST_LOG=info cargo run --bin miden-proving-service $(PROVER_FEATURES_TESTING) --release --locked -- start-worker --port $(PROVER_PORT) --prover-type transaction

.PHONY: kill-prover
kill-prover: ## Kill prover process
	pkill miden-tx-prover || echo 'process not running'

# --- Installing ----------------------------------------------------------------------------------

install: ## Install the CLI binary
	cargo install $(FEATURES_CLI) --path bin/miden-cli --locked

# --- Building ------------------------------------------------------------------------------------

build: ## Build the CLI binary and client library in release mode
	CODEGEN=1 cargo build --workspace --exclude miden-client-web --release $(FEATURES_CLI)

build-wasm: ## Build the client library for wasm32
	CODEGEN=1 cargo build --package miden-client-web --target wasm32-unknown-unknown $(FEATURES_WEB_CLIENT)

# --- Check ---------------------------------------------------------------------------------------

.PHONY: check
check: ## Build the CLI binary and client library in release mode
	cargo check --workspace --exclude miden-client-web --release $(FEATURES_CLI)

.PHONY: check-wasm
check-wasm: ## Build the client library for wasm32
	cargo check --package miden-client-web --target wasm32-unknown-unknown $(FEATURES_WEB_CLIENT)
