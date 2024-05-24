.DEFAULT_GOAL := help

.PHONY: help
help: ## Show description of all commands
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# --- Variables ----------------------------------------------------------------------------------------

FEATURES_INTEGRATION_TESTING="integration"
NODE_FEATURES_TESTING="testing"

# --- Testing ----------------------------------------------------------------------------------------
.PHONY: test
test: ## Run tests
	cargo nextest run --release --workspace

.PHONY: ci-test
ci-test: ## Run tests with CI profile
	cargo nextest run --profile ci-default --release --workspace

# --- Integration testing ----------------------------------------------------------------------------------------
.PHONY: integration-test
integration-test: ## Run integration tests
	cargo nextest run --release --test=integration --features $(FEATURES_INTEGRATION_TESTING)


.PHONY: ci-integration-test
ci-integration-test: ## Run integration tests with CI profile
	cargo nextest run --profile ci-default --release --test=integration --features $(FEATURES_INTEGRATION_TESTING)

.PHONY: kill-node
kill-node: ## Kill node process
	pkill miden-node || echo 'process not running'

.PHONY: clean-node
clean-node: ## Clean node directory
	rm -rf miden-node

.PHONY: node
node: ## Setup node
	if [ -d miden-node ]; then cd miden-node ; else git clone https://github.com/0xPolygonMiden/miden-node.git && cd miden-node; fi
	cd miden-node && git checkout main && git pull origin main && cargo update
	cd miden-node && rm -rf miden-store.sqlite3 miden-store.sqlite3-wal miden-store.sqlite3-shm
	cd miden-node && cargo run --bin miden-node --features $(NODE_FEATURES_TESTING) -- make-genesis --inputs-path ../tests/config/genesis.toml --force

.PHONY: start-node
start-node: ## Run node
	cd miden-node && cargo run --bin miden-node --features $(NODE_FEATURES_TESTING) -- start --config ../tests/config/miden-node.toml node

# --- Linting ----------------------------------------------------------------------------------------

.PHONY: format
format: ## Runs format using nightly toolchain
	cargo +nightly fmt --all

.PHONY: format-check
format-check: ## Runs format using nightly toolchain but only in check mode
	cargo +nightly fmt --all --check

.PHONY: clippy
clippy: ## Runs clippy on all targets (except integration tests) with config
	cargo clippy --workspace --all-targets -- -D clippy::all -D warnings

.PHONY: clippy-integration-tests
clippy-integration-tests: ## Runs clippy integration tests with config
	cargo clippy --workspace --tests --features integration -- -D clippy::all -D warnings

.PHONY: clippy-all
clippy-all: clippy clippy-integration-tests ## Runs over all targets

.PHONY: lint
lint: check-format clippy-all doc ## Runs all linting tasks at once (clippy, formatting, doc)

# --- Documentation site ----------------------------------------------------------------------------------------
.PHONY: doc-deps
doc-deps: ## Install dependencies to build and serve documentation site
	pip3 install -r scripts/docs_requirements.txt

.PHONY: doc-build
doc-build: doc-deps ## Build documentation site
	mkdocs build

.PHONY: doc-serve
doc-serve: doc-deps ## Serve documentation site
	mkdocs serve

# --- Rust documentation ----------------------------------------------------------------------------------------
.PHONY: doc
doc: ## Generates & checks rust documentation
	cargo doc --all--features --keep-going --release
